use crate::ir::{lower_ast_to_ir, IrFunction, IrProgram};
use crate::lexer::scan_tokens;
use crate::observability::ObservabilityRun;
use crate::parser::parse_ast;
use crate::resolver::{resolve_ast_with_externals, ExternalModules};
use crate::runtime::{evaluate_named_function, RuntimeValue};
use crate::typing::infer_types;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

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

#[derive(Default)]
struct ReplSession {
    accumulated_functions: Vec<IrFunction>,
    external_modules: ExternalModules,
}

struct ReplTypeInfo {
    inferred_signatures: usize,
    runtime_type: Option<&'static str>,
}

enum ReplMode {
    Interactive,
    Server { listen_addr: String },
}

enum ReadResult {
    Eof,
    Empty,
    Quit,
    Command(String),
    Input(String),
}

#[derive(Debug, Deserialize)]
struct ServerRequest {
    op: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServerResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl ServerResponse {
    fn ok_value(value: &RuntimeValue) -> Self {
        Self {
            status: "ok",
            value: Some(value.render()),
            value_type: Some(value_type_label(value).to_string()),
            message: None,
        }
    }

    fn ok_message(message: impl Into<String>) -> Self {
        Self {
            status: "ok",
            value: None,
            value_type: None,
            message: Some(message.into()),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error",
            value: None,
            value_type: None,
            message: Some(message.into()),
        }
    }
}

impl ReplSession {
    fn clear(&mut self) {
        self.accumulated_functions.clear();
        self.external_modules.clear();
    }

    fn eval_source(&mut self, source: &str) -> Result<RuntimeValue, String> {
        let wrapped = wrap_expr_in_module(source);
        compile_and_run(
            &wrapped,
            &mut self.accumulated_functions,
            &mut self.external_modules,
        )
    }

    fn load_file(&mut self, path: &str) -> Result<RuntimeValue, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|err| format!("could not read '{}': {err}", path))?;
        self.eval_source(&source)
    }

    fn infer_expr_type(&self, expr: &str) -> Result<ReplTypeInfo, String> {
        let wrapped = wrap_expr_in_module(expr);
        let tokens = scan_tokens(&wrapped).map_err(|err| err.to_string())?;
        let ast = parse_ast(&tokens).map_err(|err| err.to_string())?;
        resolve_ast_with_externals(&ast, &self.external_modules).map_err(|err| err.to_string())?;
        let summary = infer_types(&ast).map_err(|err| err.to_string())?;

        let runtime_type = lower_ast_to_ir(&ast)
            .ok()
            .and_then(|ir| {
                let mut full_functions: Vec<IrFunction> = self.accumulated_functions.clone();
                full_functions.extend(ir.functions);
                let program = IrProgram {
                    functions: full_functions,
                };
                evaluate_named_function(&program, REPL_FN).ok()
            })
            .map(|value| value_type_label(&value));

        Ok(ReplTypeInfo {
            inferred_signatures: summary.len(),
            runtime_type,
        })
    }
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

fn handle_command(cmd: &str, session: &ReplSession) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    match parts[0] {
        ":help" | ":h" => print_help(),
        ":quit" | ":q" => {} // handled by caller
        ":clear" | ":c" => {
            // Can't mutate here; caller must handle clear.
            // We print a note since clear is handled in the loop via a mutable ref.
            // Because handle_command takes &ReplSession, the caller will detect :clear via
            // ReadResult::Command and handle it there. This branch is a fallback.
            eprintln!("note: use :clear to reset the environment");
        }
        ":type" => {
            if let Some(expr) = parts.get(1) {
                handle_type_command(expr.trim(), session);
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

fn handle_type_command(expr: &str, session: &ReplSession) {
    match session.infer_expr_type(expr) {
        Ok(info) => {
            if info.inferred_signatures > 0 {
                println!(
                    "dynamic (inferred {} signature(s))",
                    info.inferred_signatures
                );
            } else {
                println!("dynamic");
            }
            if let Some(runtime_type) = info.runtime_type {
                println!("runtime type: {runtime_type}");
            }
        }
        Err(err) => eprintln!("error: {err}"),
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
        RuntimeValue::Binary(_) => "binary",
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

fn process_input(source: &str, session: &mut ReplSession) {
    match session.eval_source(source) {
        Ok(RuntimeValue::Nil) => {}
        Ok(value) => println!("{}", value.render()),
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
/// function call. If it looks like a plain expression, we wrap it directly.
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

fn parse_repl_mode(args: &[String]) -> Result<ReplMode, crate::cli_diag::CliDiagnostic> {
    let mut iter = args.iter();
    let mut listen_addr = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(ReplMode::Interactive);
            }
            "--listen" => {
                let value = iter.next().ok_or_else(|| {
                    crate::cli_diag::CliDiagnostic::usage_with_hint(
                        "missing value for --listen",
                        "run `tonic repl --listen 127.0.0.1:7888`",
                    )
                })?;
                listen_addr = Some(value.clone());
            }
            other => {
                return Err(crate::cli_diag::CliDiagnostic::usage_with_hint(
                    format!("unknown repl argument '{other}'"),
                    "run `tonic repl --help` to see available options",
                ));
            }
        }
    }

    Ok(match listen_addr {
        Some(listen_addr) => ReplMode::Server { listen_addr },
        None => ReplMode::Interactive,
    })
}

fn print_usage() {
    println!(
        "Usage:\n  tonic repl [--listen <addr>]\n\nStart an interactive Tonic REPL or a socket-driven REPL server.\n"
    );
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

fn run_repl_mode(mode: &ReplMode) -> Result<(), String> {
    match mode {
        ReplMode::Interactive => run_repl_with_clear(),
        ReplMode::Server { listen_addr } => run_repl_server(listen_addr),
    }
}

// --- Entry point wired from main.rs ---

pub fn handle_repl(args: Vec<String>) -> i32 {
    let mode = match parse_repl_mode(&args) {
        Ok(mode) => mode,
        Err(diag) => return diag.emit(),
    };

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        return crate::cli_diag::EXIT_OK;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("repl".to_string());
    argv.extend(args.iter().cloned());
    let mut observed_run = ObservabilityRun::from_env("repl", &argv, &cwd);

    let run_result = if let Some(observed_run) = observed_run.as_mut() {
        observed_run.phase_result("repl.session", || run_repl_mode(&mode))
    } else {
        run_repl_mode(&mode)
    };

    let exit_code = match run_result {
        Ok(()) => crate::cli_diag::EXIT_OK,
        Err(err) => {
            eprintln!("error: {err}");
            crate::cli_diag::EXIT_FAILURE
        }
    };

    if let Some(observed_run) = observed_run.as_mut() {
        for warning in observed_run.finish_with_status(exit_code, None) {
            eprintln!("warning: {warning}");
        }
    }
    exit_code
}

/// Extended loop that handles :clear by resetting accumulated state.
fn run_repl_with_clear() -> Result<(), String> {
    println!("{WELCOME}");

    let mut rl =
        DefaultEditor::new().map_err(|err| format!("could not create readline editor: {err}"))?;

    let history_path = history_file_path();
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    let mut session = ReplSession::default();

    loop {
        match read_input(&mut rl) {
            ReadResult::Eof => break,
            ReadResult::Empty => continue,
            ReadResult::Quit => break,
            ReadResult::Command(ref cmd) if matches!(cmd.as_str(), ":clear" | ":c") => {
                session.clear();
                println!("environment cleared");
            }
            ReadResult::Command(cmd) => {
                handle_command(&cmd, &session);
            }
            ReadResult::Input(source) => {
                process_input(&source, &mut session);
            }
        }
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn run_repl_server(listen_addr: &str) -> Result<(), String> {
    let listener = TcpListener::bind(listen_addr)
        .map_err(|err| format!("could not bind REPL server on {listen_addr}: {err}"))?;
    let bound_addr = listener
        .local_addr()
        .map_err(|err| format!("could not inspect bound REPL address: {err}"))?;

    println!("Tonic REPL server listening on {bound_addr}");
    std::io::stdout()
        .flush()
        .map_err(|err| format!("could not flush REPL server banner: {err}"))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(err) = handle_repl_client(stream) {
                        eprintln!("warning: repl client session ended with error: {err}");
                    }
                });
            }
            Err(err) => eprintln!("warning: failed to accept repl client: {err}"),
        }
    }

    Ok(())
}

fn handle_repl_client(stream: TcpStream) -> Result<(), String> {
    let reader_stream = stream
        .try_clone()
        .map_err(|err| format!("could not clone repl client stream: {err}"))?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;
    let mut session = ReplSession::default();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|err| format!("could not read repl request: {err}"))?;
        if bytes_read == 0 {
            return Ok(());
        }

        let request_line = line.trim();
        if request_line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<ServerRequest>(request_line) {
            Ok(request) => handle_server_request(&mut session, request),
            Err(err) => ServerResponse::error(format!("invalid request JSON: {err}")),
        };

        write_server_response(&mut writer, &response)
            .map_err(|err| format!("could not write repl response: {err}"))?;
    }
}

fn handle_server_request(session: &mut ReplSession, request: ServerRequest) -> ServerResponse {
    match request.op.as_str() {
        "eval" => match request.code.as_deref() {
            Some(code) => match session.eval_source(code) {
                Ok(value) => ServerResponse::ok_value(&value),
                Err(err) => ServerResponse::error(err),
            },
            None => ServerResponse::error("missing 'code' for eval request"),
        },
        "clear" => {
            session.clear();
            ServerResponse::ok_message("environment cleared")
        }
        "load-file" => match request.path.as_deref() {
            Some(path) => match session.load_file(path) {
                Ok(value) => ServerResponse::ok_value(&value),
                Err(err) => ServerResponse::error(err),
            },
            None => ServerResponse::error("missing 'path' for load-file request"),
        },
        other => ServerResponse::error(format!(
            "unknown op '{other}' (expected eval, clear, or load-file)"
        )),
    }
}

fn write_server_response(writer: &mut TcpStream, response: &ServerResponse) -> std::io::Result<()> {
    serde_json::to_writer(&mut *writer, response)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    writer.write_all(b"\n")?;
    writer.flush()
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

        let call = wrap_expr_in_module("Helpers.double(5)");
        let result = compile_and_run(call.as_str(), &mut acc, &mut externals);
        assert_eq!(result.unwrap(), RuntimeValue::Int(10));
    }

    #[test]
    fn repl_session_clear_resets_external_definitions() {
        let mut session = ReplSession::default();
        session
            .eval_source("defmodule Helpers do\n  def double(x) do x * 2 end\nend")
            .expect("definition should succeed");
        let value = session
            .eval_source("Helpers.double(5)")
            .expect("helper should be available before clear");
        assert_eq!(value, RuntimeValue::Int(10));

        session.clear();

        let error = session
            .eval_source("Helpers.double(5)")
            .expect_err("helper should not be available after clear");
        assert!(error.contains("Helpers"));
    }

    #[test]
    fn handle_server_request_supports_eval_clear_and_load_file() {
        let mut session = ReplSession::default();
        let eval = handle_server_request(
            &mut session,
            ServerRequest {
                op: "eval".to_string(),
                code: Some("1 + 2".to_string()),
                path: None,
            },
        );
        assert_eq!(eval.status, "ok");
        assert_eq!(eval.value.as_deref(), Some("3"));
        assert_eq!(eval.value_type.as_deref(), Some("int"));

        let temp_path = std::env::temp_dir().join(format!(
            "tonic-repl-load-file-{}-{}.tn",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        std::fs::write(
            &temp_path,
            "defmodule Helpers do\n  def double(x) do x * 2 end\nend\n",
        )
        .expect("temp file should be writable");

        let load_file = handle_server_request(
            &mut session,
            ServerRequest {
                op: "load-file".to_string(),
                code: None,
                path: Some(temp_path.display().to_string()),
            },
        );
        assert_eq!(load_file.status, "ok");
        assert_eq!(load_file.value_type.as_deref(), Some("nil"));

        let persisted = handle_server_request(
            &mut session,
            ServerRequest {
                op: "eval".to_string(),
                code: Some("Helpers.double(6)".to_string()),
                path: None,
            },
        );
        assert_eq!(persisted.status, "ok");
        assert_eq!(persisted.value.as_deref(), Some("12"));

        let clear = handle_server_request(
            &mut session,
            ServerRequest {
                op: "clear".to_string(),
                code: None,
                path: None,
            },
        );
        assert_eq!(clear.status, "ok");
        assert_eq!(clear.message.as_deref(), Some("environment cleared"));

        let missing = handle_server_request(
            &mut session,
            ServerRequest {
                op: "eval".to_string(),
                code: Some("Helpers.double(6)".to_string()),
                path: None,
            },
        );
        assert_eq!(missing.status, "error");

        let _ = std::fs::remove_file(temp_path);
    }
}
