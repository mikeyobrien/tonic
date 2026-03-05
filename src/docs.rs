use crate::cli_diag::{CliDiagnostic, EXIT_OK};
use crate::lexer::scan_tokens;
use crate::manifest::{load_run_source, STDLIB_SOURCES};
use crate::parser::{parse_ast, Expr, Module};
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT_DIR: &str = "docs/api";

/// Extract the string value from an `Expr`, if it is a plain string literal.
fn expr_as_str(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::String { value, .. } => Some(value.as_str()),
        _ => None,
    }
}

/// Find the `@moduledoc` attribute value in a module's attribute list.
fn module_doc(module: &Module) -> Option<&str> {
    module
        .attributes
        .iter()
        .find(|attr| attr.name == "moduledoc")
        .and_then(|attr| expr_as_str(&attr.value))
}

/// One-line summary: first sentence of the moduledoc (up to the first `.`).
fn module_summary(module: &Module) -> Option<String> {
    module_doc(module).map(|doc| {
        doc.split('.')
            .next()
            .map(|s| format!("{}.", s.trim()))
            .unwrap_or_else(|| doc.to_string())
    })
}

/// Build a list of (function_name, arity, param_names, doc_string_or_None) entries
/// for **public** functions only.
///
/// Strategy: `@doc` attributes are matched to public functions in declaration
/// order. The Nth `@doc` attribute is associated with the Nth public function.
fn function_docs(module: &Module) -> Vec<(String, usize, Vec<String>, Option<String>)> {
    let doc_attrs: Vec<&str> = module
        .attributes
        .iter()
        .filter(|attr| attr.name == "doc")
        .filter_map(|attr| expr_as_str(&attr.value))
        .collect();

    let mut doc_iter = doc_attrs.into_iter();

    module
        .functions
        .iter()
        .filter(|func| !func.is_private())
        .map(|func| {
            let param_names: Vec<String> = func
                .params
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let arity = param_names.len();
            let doc = doc_iter.next().map(|s| s.to_string());
            (func.name.clone(), arity, param_names, doc)
        })
        .collect()
}

/// Render Markdown documentation for a single module.
pub fn render_module_doc(module: &Module) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n", module.name));

    if let Some(doc) = module_doc(module) {
        out.push('\n');
        out.push_str(doc);
        out.push('\n');
    }

    // function_docs already filters to public functions only
    let funcs = function_docs(module);

    if !funcs.is_empty() {
        out.push_str("\n## Functions\n");

        for (name, arity, params, doc) in &funcs {
            let signature = if params.is_empty() {
                format!("{}()", name)
            } else {
                format!("{}({})", name, params.join(", "))
            };

            out.push('\n');
            out.push_str(&format!("### {}/{}\n", name, arity));
            out.push('\n');
            out.push_str(&format!("```tonic\ndef {signature}\n```\n"));

            if let Some(doc_str) = doc {
                out.push('\n');
                out.push_str(doc_str);
                out.push('\n');
            }
        }
    }

    out
}

/// Write per-module `.md` files and an `index.md` to `output_dir`.
///
/// Returns `(module_count, function_count)`.
fn write_docs_to_dir(
    modules: &[Module],
    output_dir: &Path,
    builtin: bool,
) -> Result<(usize, usize), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "failed to create output directory {}: {}",
            output_dir.display(),
            e
        )
    })?;

    let mut total_functions = 0usize;
    let mut module_summaries: Vec<(String, Option<String>)> = Vec::new();

    for module in modules {
        total_functions += module
            .functions
            .iter()
            .filter(|f| !f.is_private())
            .count();

        let content = render_module_doc(module);
        let filename = format!("{}.md", module.name.to_lowercase());
        let file_path = output_dir.join(&filename);

        std::fs::write(&file_path, &content).map_err(|e| {
            format!("failed to write {}: {}", file_path.display(), e)
        })?;

        module_summaries.push((module.name.clone(), module_summary(module)));
    }

    // Write index.md
    let index_path = output_dir.join("index.md");
    let index_content = render_index(&module_summaries, builtin);
    std::fs::write(&index_path, &index_content)
        .map_err(|e| format!("failed to write {}: {}", index_path.display(), e))?;

    Ok((modules.len(), total_functions))
}

/// Render the index.md listing all modules.
fn render_index(summaries: &[(String, Option<String>)], builtin: bool) -> String {
    let mut out = String::new();

    if builtin {
        out.push_str("# Built-in Modules\n\n");
        out.push_str("> These modules are part of the Tonic standard library.\n\n");
    } else {
        out.push_str("# Module Index\n\n");
    }

    out.push_str("| Module | Summary |\n");
    out.push_str("|---|---|\n");

    for (name, summary) in summaries {
        let summary_str = summary.as_deref().unwrap_or("—");
        let link = format!("[{}]({}.md)", name, name.to_lowercase());
        out.push_str(&format!("| {} | {} |\n", link, summary_str));
    }

    out
}

/// Parse source text into modules, returning an empty vec on parse failure.
fn parse_modules(source: &str) -> Vec<Module> {
    let Ok(tokens) = scan_tokens(source) else {
        return Vec::new();
    };
    let Ok(ast) = parse_ast(&tokens) else {
        return Vec::new();
    };
    ast.modules
}

/// Entry point for `tonic docs <path> [--output <dir>]`.
pub fn handle_docs(args: Vec<String>) -> i32 {
    if args.is_empty()
        || matches!(args.first().map(String::as_str), Some("-h") | Some("--help"))
    {
        if args.is_empty() {
            return CliDiagnostic::usage_with_hint(
                "missing required <path>",
                "run `tonic docs --help` for usage",
            )
            .emit();
        }
        print_docs_help();
        return EXIT_OK;
    }

    let source_path = &args[0];
    let mut output_dir: Option<String> = None;
    let mut idx = 1;

    while idx < args.len() {
        match args[idx].as_str() {
            "--output" => {
                idx += 1;
                if idx >= args.len() {
                    return CliDiagnostic::usage("--output requires a value").emit();
                }
                output_dir = Some(args[idx].clone());
                idx += 1;
            }
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let source = match load_run_source(source_path) {
        Ok(src) => src,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let tokens = match scan_tokens(&source) {
        Ok(tokens) => tokens,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    let ast = match parse_ast(&tokens) {
        Ok(ast) => ast,
        Err(error) => {
            return CliDiagnostic::failure_with_source(error.to_string(), &source, error.offset())
                .emit();
        }
    };

    let out_dir = PathBuf::from(output_dir.as_deref().unwrap_or(DEFAULT_OUTPUT_DIR));

    match write_docs_to_dir(&ast.modules, &out_dir, false) {
        Ok((module_count, fn_count)) => {
            // Also generate stdlib docs in a sub-directory
            let stdlib_dir = out_dir.join("stdlib");
            let stdlib_modules = collect_stdlib_modules();
            let stdlib_result = write_docs_to_dir(&stdlib_modules, &stdlib_dir, true);

            let (stdlib_mod_count, stdlib_fn_count) = match stdlib_result {
                Ok(counts) => counts,
                Err(e) => {
                    eprintln!("warning: stdlib docs generation failed: {e}");
                    (0, 0)
                }
            };

            let total_modules = module_count + stdlib_mod_count;
            let total_fns = fn_count + stdlib_fn_count;

            println!(
                "Generated docs for {} module{} ({} function{}) in {}",
                total_modules,
                if total_modules == 1 { "" } else { "s" },
                total_fns,
                if total_fns == 1 { "" } else { "s" },
                out_dir.display()
            );
        }
        Err(error) => return CliDiagnostic::failure(error).emit(),
    }

    EXIT_OK
}

/// Collect all stdlib modules by parsing their embedded sources.
fn collect_stdlib_modules() -> Vec<Module> {
    STDLIB_SOURCES
        .iter()
        .flat_map(|(_, source)| parse_modules(source))
        .collect()
}

fn print_docs_help() {
    println!(
        "Usage:\n  tonic docs <path> [--output <dir>]\n\n\
         Options:\n  --output <dir>   Output directory (default: {})\n\n\
         Generates Markdown documentation for all modules in <path>.\n\
         Also generates docs for built-in stdlib modules in <output>/stdlib/.\n",
        DEFAULT_OUTPUT_DIR
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;

    fn parse_source(src: &str) -> Vec<Module> {
        let tokens = scan_tokens(src).expect("lex should succeed");
        parse_ast(&tokens).expect("parse should succeed").modules
    }

    #[test]
    fn module_doc_returns_none_when_no_moduledoc() {
        let modules = parse_source("defmodule Bare do\n  def run() do 1 end\nend\n");
        assert_eq!(module_doc(&modules[0]), None);
    }

    #[test]
    fn module_doc_extracts_moduledoc_string() {
        let modules = parse_source(
            "defmodule Documented do\n  @moduledoc \"A great module.\"\n  def run() do 1 end\nend\n",
        );
        assert_eq!(module_doc(&modules[0]), Some("A great module."));
    }

    #[test]
    fn function_docs_extracts_doc_strings_in_order() {
        let modules = parse_source(
            r#"defmodule M do
  @doc "First doc."
  def first(a) do a end
  @doc "Second doc."
  def second(b, c) do b end
end
"#,
        );
        let funcs = function_docs(&modules[0]);
        assert_eq!(funcs.len(), 2);
        assert_eq!(funcs[0].0, "first");
        assert_eq!(funcs[0].1, 1);
        assert_eq!(funcs[0].3.as_deref(), Some("First doc."));
        assert_eq!(funcs[1].0, "second");
        assert_eq!(funcs[1].1, 2);
        assert_eq!(funcs[1].3.as_deref(), Some("Second doc."));
    }

    #[test]
    fn function_docs_skips_doc_for_private_functions() {
        let modules = parse_source(
            r#"defmodule M do
  @doc "Public doc."
  def pub_fn(a) do a end
  defp priv_fn(b) do b end
end
"#,
        );
        let funcs = function_docs(&modules[0]);
        // Private function should not consume the @doc
        let pub_entry = funcs.iter().find(|(n, _, _, _)| n == "pub_fn").unwrap();
        assert_eq!(pub_entry.3.as_deref(), Some("Public doc."));
    }

    #[test]
    fn render_module_doc_contains_module_name() {
        let modules = parse_source("defmodule Hello do\n  def run() do 1 end\nend\n");
        let output = render_module_doc(&modules[0]);
        assert!(output.contains("# Hello"), "expected '# Hello' in:\n{output}");
    }

    #[test]
    fn render_module_doc_includes_function_signature() {
        let modules = parse_source(
            "defmodule M do\n  def add(a, b) do\n    a + b\n  end\nend\n",
        );
        let output = render_module_doc(&modules[0]);
        assert!(output.contains("add(a, b)"), "expected signature in:\n{output}");
    }

    #[test]
    fn render_module_doc_includes_arity_in_heading() {
        let modules = parse_source(
            "defmodule M do\n  def add(a, b) do\n    a + b\n  end\nend\n",
        );
        let output = render_module_doc(&modules[0]);
        assert!(output.contains("### add/2"), "expected 'add/2' heading in:\n{output}");
    }

    #[test]
    fn render_index_contains_all_module_names() {
        let summaries = vec![
            ("Alpha".to_string(), Some("Alpha module.".to_string())),
            ("Beta".to_string(), None),
        ];
        let index = render_index(&summaries, false);
        assert!(index.contains("Alpha"), "expected Alpha in:\n{index}");
        assert!(index.contains("Beta"), "expected Beta in:\n{index}");
    }

    #[test]
    fn render_index_marks_builtin_modules() {
        let summaries = vec![("System".to_string(), None)];
        let index = render_index(&summaries, true);
        assert!(
            index.contains("Built-in"),
            "expected 'Built-in' marker in:\n{index}"
        );
    }

    #[test]
    fn write_docs_to_dir_creates_module_files() {
        let modules = parse_source(
            r#"defmodule Greeter do
  @moduledoc "Greets people."
  def greet(name) do name end
end
"#,
        );
        let tmp = std::env::temp_dir().join(format!(
            "tonic-docs-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let (mod_count, fn_count) = write_docs_to_dir(&modules, &tmp, false).unwrap();
        assert_eq!(mod_count, 1);
        assert_eq!(fn_count, 1);

        let module_file = tmp.join("greeter.md");
        assert!(module_file.exists(), "greeter.md should be created");
        let content = std::fs::read_to_string(&module_file).unwrap();
        assert!(content.contains("Greeter"));
        assert!(content.contains("Greets people."));

        let index_file = tmp.join("index.md");
        assert!(index_file.exists(), "index.md should be created");
    }

    #[test]
    fn collect_stdlib_modules_returns_system_and_enum() {
        let modules = collect_stdlib_modules();
        let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"System"), "expected System in {names:?}");
        assert!(names.contains(&"Enum"), "expected Enum in {names:?}");
    }

    #[test]
    fn module_summary_returns_first_sentence() {
        let modules = parse_source(
            "defmodule M do\n  @moduledoc \"First sentence. Second sentence.\"\n  def run() do 1 end\nend\n",
        );
        let summary = module_summary(&modules[0]);
        assert_eq!(summary.as_deref(), Some("First sentence."));
    }
}
