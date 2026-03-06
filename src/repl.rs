use crate::ir::{lower_ast_to_ir, IrFunction, IrProgram};
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;
use crate::resolver::{resolve_ast_with_externals, ExternalModules};
use crate::runtime::{evaluate_named_function, RuntimeValue};
use crate::typing::infer_types;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

const WELCOME: &str = concat!(
    "Tonic v",
    env!("CARGO_PKG_VERSION"),
    " (type :help for commands, :quit to exit)"
);
const PROMPT_NORMAL: &str = "tonic> ";
const PROMPT_CONT: &str = "  ...> ";
const HISTORY_FILE: &str = ".tonic_history";
const REPL_MODULE: &str = "Repl";
const REPL_FN: &str = "Repl.__repl_entry__";

// Tokens that indicate a block is open and more lines are expected.
const BLOCK_OPENERS: &[&str] = &["do", "fn", "->", "(", "[", "{", "\\"];

enum ReadResult {
    Eof,
    Empty,
    Quit,
    Command(String),
    Input(String),
}

fn read_input(rl: &mut DefaultEditor) -> ReadResult {
    let first = match rl.readline(PROMPT_NORMAL) {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) => return ReadResult::Empty,
        Err(ReadlineError::Eof) => return ReadResult::Eof,
        Err(err) => {
            eprintln!("error: {err}");
            return ReadResult::Eof;
        }
    };

    let trimmed = first.trim();

    if trimmed.is_empty() {
        return ReadResult::Empty;
    }

    // Special commands start with ':'
    if trimmed.starts_with(':') {
        let _ = rl.add_history_entry(trimmed);
        if is_quit_command(trimmed) {
            return ReadResult::Quit;
        }
        return ReadResult::Command(trimmed.to_string());
    }

    let _ = rl.add_history_entry(trimmed);

    // Multiline continuation: keep reading while the block is open.
    let mut lines = vec![first.clone()];
    while needs_continuation(&lines) {
        match rl.readline(PROMPT_CONT) {
            Ok(cont) => {
                let _ = rl.add_history_entry(cont.trim());
                lines.push(cont);
            }
            Err(ReadlineError::Interrupted) => break,
            Err(_) => break,
        }
    }

    ReadResult::Input(lines.join("\n"))
}

fn is_quit_command(cmd: &str) -> bool {
    matches!(cmd, ":quit" | ":q")
}

/// Returns true when the accumulated lines look like an open block.
fn needs_continuation(lines: &[String]) -> bool {
    let joined = lines.join(" ");
    let trimmed = joined.trim();

    // Check if the last non-empty token ends with an opener keyword/symbol.
    for opener in BLOCK_OPENERS {
        if trimmed.ends_with(opener) {
            return true;
        }
    }

    // Balance check for brackets: more openers than closers means we need more input.
    let opens = count_chars(trimmed, &['{', '(', '[']);
    let closes = count_chars(trimmed, &['}', ')', ']']);
    if opens > closes {
        return true;
    }

    // Balance check for do/end blocks (word-boundary aware).
    let do_count = count_keyword(trimmed, "do");
    let fn_count = count_keyword(trimmed, "fn");
    let end_count = count_keyword(trimmed, "end");
    (do_count + fn_count) > end_count
}

fn count_chars(s: &str, targets: &[char]) -> usize {
    s.chars().filter(|c| targets.contains(c)).count()
}

/// Count occurrences of a keyword at word boundaries (not inside identifiers).
fn count_keyword(s: &str, keyword: &str) -> usize {
    s.split_whitespace().filter(|word| *word == keyword).count()
}

fn handle_command(cmd: &str, accumulated: &[IrFunction]) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    match parts[0] {
        ":help" | ":h" => print_help(),
        ":quit" | ":q" => {} // handled by caller
        ":clear" | ":c" => {
            // Can't mutate here; caller must handle clear.
            // We print a note since clear is handled in the loop via a mutable ref.
            // Because handle_command takes &Vec, the caller will detect :clear via
            // ReadResult::Command and handle it there. This branch is a fallback.
            eprintln!("note: use :clear to reset the environment");
        }
        ":type" => {
            if let Some(expr) = parts.get(1) {
                handle_type_command(expr.trim(), accumulated);
            } else {
                eprintln!("usage: :type <expression>");
            }
        }
        other => {
            eprintln!(
                "unknown command '{}' — type :help for available commands",
                other
            );
        }
    }
}

fn handle_type_command(expr: &str, accumulated: &[IrFunction]) {
    // Wrap expression in a module function for type inference.
    let wrapped = wrap_expr_in_module(expr);
    let tokens = match scan_tokens(&wrapped) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("error: {err}");
            return;
        }
    };
    let ast = match parse_ast(&tokens) {
        Ok(a) => a,
        Err(err) => {
            eprintln!("error: {err}");
            return;
        }
    };
    if let Err(err) = resolve_ast_with_externals(&ast, &ExternalModules::new()) {
        eprintln!("error: {err}");
        return;
    }
    match infer_types(&ast) {
        Ok(summary) => {
            // Report the accumulated function count from the summary
            let count = summary.len();
            if count > 0 {
                println!("dynamic (inferred {} signature(s))", count);
            } else {
                println!("dynamic");
            }
            // Evaluate to get the runtime type tag
            if let Ok(ir) = lower_ast_to_ir(&ast) {
                let mut full_functions: Vec<IrFunction> = accumulated.to_vec();
                full_functions.extend(ir.functions);
                let program = IrProgram {
                    functions: full_functions,
                };
                if let Ok(value) = evaluate_named_function(&program, REPL_FN) {
                    println!("runtime type: {}", value_type_label(&value));
                }
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
        }
    }
}

fn value_type_label(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "int",
        RuntimeValue::Float(_) => "float",
        RuntimeValue::Bool(_) => "bool",
        RuntimeValue::Nil => "nil",
        RuntimeValue::String(_) => "string",
        RuntimeValue::Atom(_) => "atom",
        RuntimeValue::ResultOk(_) => "ok(_)",
        RuntimeValue::ResultErr(_) => "err(_)",
        RuntimeValue::Tuple(_, _) => "{_, _}",
        RuntimeValue::Map(_) => "map",
        RuntimeValue::Keyword(_) => "keyword",
        RuntimeValue::List(_) => "list",
        RuntimeValue::Range(_, _) => "range",
        RuntimeValue::SteppedRange(_, _, _) => "range",
        RuntimeValue::Closure(_) => "function",
    }
}

fn extract_module_signatures(ast: &crate::parser::Ast) -> ExternalModules {
    let mut modules = ExternalModules::new();
    for module in &ast.modules {
        if module.name == REPL_MODULE {
            continue;
        }
        let mut fns = std::collections::HashMap::new();
        for func in &module.functions {
            fns.insert(func.name.clone(), !func.is_private());
        }
        modules.insert(module.name.clone(), fns);
    }
    modules
}

fn process_input(
    source: &str,
    accumulated: &mut Vec<IrFunction>,
    external_modules: &mut ExternalModules,
) {
    let wrapped = wrap_expr_in_module(source);
    let result = compile_and_run(&wrapped, accumulated, external_modules);
    match result {
        Ok(value) => match value {
            RuntimeValue::Nil => {} // suppress nil output for definitions
            other => println!("{}", other.render()),
        },
        Err(msg) => eprintln!("error: {msg}"),
    }
}

fn compile_and_run(
    source: &str,
    accumulated: &mut Vec<IrFunction>,
    external_modules: &mut ExternalModules,
) -> Result<RuntimeValue, String> {
    let tokens = scan_tokens(source).map_err(|e| e.to_string())?;
    let ast = parse_ast(&tokens).map_err(|e| e.to_string())?;
    resolve_ast_with_externals(&ast, external_modules).map_err(|e| e.to_string())?;
    infer_types(&ast).map_err(|e| e.to_string())?;
    let ir = lower_ast_to_ir(&ast).map_err(|e| e.to_string())?;

    // Extract and accumulate module signatures from this AST before mutating accumulated.
    let new_signatures = extract_module_signatures(&ast);
    for (mod_name, fns) in new_signatures {
        external_modules.insert(mod_name, fns);
    }

    // Merge: new functions override same-named accumulated ones.
    let new_names: std::collections::HashSet<&str> =
        ir.functions.iter().map(|f| f.name.as_str()).collect();
    accumulated.retain(|f| !new_names.contains(f.name.as_str()));
    accumulated.extend(ir.functions);

    // Build complete program with all accumulated functions.
    let program = IrProgram {
        functions: accumulated.clone(),
    };

    evaluate_named_function(&program, REPL_FN).map_err(|e| e.to_string())
}

/// Wraps arbitrary source (expression or module definition) inside a module
/// function that the runtime can call as the REPL entry point.
///
/// If the source defines a module, we append the entry wrapper as an extra
/// function call.  If it looks like a plain expression, we wrap it directly.
fn wrap_expr_in_module(source: &str) -> String {
    let trimmed = source.trim();

    // If the input already contains a module definition, append a thin
    // wrapper module that delegates to the entry point.
    if looks_like_module_definition(trimmed) {
        format!(
            "{trimmed}\n\ndefmodule {REPL_MODULE} do\n  def __repl_entry__() do\n    nil\n  end\nend\n"
        )
    } else {
        format!(
            "defmodule {REPL_MODULE} do\n  def __repl_entry__() do\n    {trimmed}\n  end\nend\n"
        )
    }
}

fn looks_like_module_definition(source: &str) -> bool {
    source.trim_start().starts_with("defmodule ")
}

fn history_file_path() -> Option<std::path::PathBuf> {
    dirs_path().map(|mut p| {
        p.push(HISTORY_FILE);
        p
    })
}

fn dirs_path() -> Option<std::path::PathBuf> {
    // Try HOME env first; fall back to current dir.
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| Some(std::path::PathBuf::from(".")))
}

fn print_help() {
    println!(
        "\nREPL commands:\n\
         \x20 :help, :h          show this message\n\
         \x20 :quit, :q          exit the REPL\n\
         \x20 :clear, :c         reset the environment\n\
         \x20 :type <expr>       show the inferred type of an expression\n\
         \nMultiline input: keep typing after do, fn, ->, (, [, {{ or \\\n"
    );
}

// --- Entry point wired from main.rs ---

pub fn handle_repl(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        println!("Usage:\n  tonic repl\n\nStart an interactive Tonic REPL.\n");
        return crate::cli_diag::EXIT_OK;
    }

    // Run the interactive loop; it exits cleanly on :quit / EOF.
    run_repl_with_clear();

    crate::cli_diag::EXIT_OK
}

/// Extended loop that handles :clear by resetting accumulated state.
fn run_repl_with_clear() {
    println!("{WELCOME}");

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("error: could not create readline editor: {err}");
            return;
        }
    };

    let history_path = history_file_path();
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    let mut accumulated_functions: Vec<IrFunction> = Vec::new();
    let mut external_modules = ExternalModules::new();

    loop {
        match read_input(&mut rl) {
            ReadResult::Eof => break,
            ReadResult::Empty => continue,
            ReadResult::Quit => break,
            ReadResult::Command(ref cmd) if matches!(cmd.as_str(), ":clear" | ":c") => {
                accumulated_functions.clear();
                external_modules.clear();
                println!("environment cleared");
            }
            ReadResult::Command(cmd) => {
                handle_command(&cmd, &accumulated_functions);
            }
            ReadResult::Input(source) => {
                process_input(&source, &mut accumulated_functions, &mut external_modules);
            }
        }
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_continuation_detects_open_brace() {
        assert!(needs_continuation(&["{".to_string()]));
        assert!(!needs_continuation(&["{}".to_string()]));
    }

    #[test]
    fn needs_continuation_detects_open_paren() {
        assert!(needs_continuation(&["fn(".to_string()]));
        assert!(!needs_continuation(&["fn()".to_string()]));
    }

    #[test]
    fn needs_continuation_detects_do_keyword() {
        assert!(needs_continuation(&["defmodule Foo do".to_string()]));
    }

    #[test]
    fn needs_continuation_balanced_is_false() {
        assert!(!needs_continuation(&["1 + 2".to_string()]));
    }

    #[test]
    fn wrap_expr_produces_callable_module() {
        let wrapped = wrap_expr_in_module("1 + 2");
        assert!(wrapped.contains("defmodule Repl do"));
        assert!(wrapped.contains("def __repl_entry__"));
        assert!(wrapped.contains("1 + 2"));
    }

    #[test]
    fn wrap_module_definition_does_not_double_wrap() {
        let src = "defmodule Foo do\n  fn bar do 1 end\nend";
        let wrapped = wrap_expr_in_module(src);
        assert!(wrapped.contains("defmodule Foo do"));
        assert!(wrapped.contains("def __repl_entry__"));
    }

    #[test]
    fn is_quit_command_recognizes_aliases() {
        assert!(is_quit_command(":quit"));
        assert!(is_quit_command(":q"));
        assert!(!is_quit_command(":help"));
    }

    #[test]
    fn compile_and_run_evaluates_simple_expression() {
        let mut acc = Vec::new();
        let mut externals = ExternalModules::new();
        let source = wrap_expr_in_module("42");
        let result = compile_and_run(&source, &mut acc, &mut externals);
        assert!(result.is_ok(), "expected ok, got: {result:?}");
        assert_eq!(result.unwrap(), RuntimeValue::Int(42));
    }

    #[test]
    fn compile_and_run_reports_syntax_error_without_panic() {
        let mut acc = Vec::new();
        let mut externals = ExternalModules::new();
        let source = wrap_expr_in_module("@@@bad syntax@@@");
        let result = compile_and_run(&source, &mut acc, &mut externals);
        assert!(result.is_err(), "expected error for bad syntax");
    }

    #[test]
    fn accumulated_functions_persist_across_calls() {
        let mut acc = Vec::new();
        let mut externals = ExternalModules::new();

        // Define a helper module.
        let def = "defmodule Helpers do\n  def double(x) do x * 2 end\nend\n\
                   defmodule Repl do\n  def __repl_entry__() do nil end\nend\n";
        let _ = compile_and_run(def, &mut acc, &mut externals);
        assert!(
            !acc.is_empty(),
            "accumulator should have functions after definition"
        );
        assert!(
            externals.contains_key("Helpers"),
            "externals should include Helpers module"
        );

        // Call the helper in a subsequent input.
        let call = wrap_expr_in_module("Helpers.double(5)");
        let result = compile_and_run(call.as_str(), &mut acc, &mut externals);
        assert_eq!(result.unwrap(), RuntimeValue::Int(10));
    }
}
