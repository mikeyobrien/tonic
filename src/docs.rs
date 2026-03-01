use crate::cli_diag::{CliDiagnostic, EXIT_OK};
use crate::lexer::scan_tokens;
use crate::manifest::load_run_source;
use crate::parser::{parse_ast, Expr, Module};

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

/// Build a list of (function_name, param_names, doc_string_or_None) entries.
///
/// The strategy mirrors how Elixir / ExDoc works: a `@doc` attribute that
/// appears *before* a function definition is associated with that function.
/// We walk the functions list in declaration order, and for each function we
/// look at the module attributes to find a `@doc` whose position in the
/// attribute slice corresponds to the function.
///
/// Because the parser stores `attributes` and `functions` as flat, ordered
/// vecs (not interleaved), we use a simple heuristic: attributes are
/// matched to functions in the same declaration order — the Nth `@doc`
/// attribute belongs to the Nth function that *has* a `@doc` entry.
fn function_docs(module: &Module) -> Vec<(String, Vec<String>, Option<String>)> {
    // Collect only @doc entries, in declaration order.
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
        .map(|func| {
            let param_names: Vec<String> = func
                .params
                .iter()
                .map(|p| p.name().to_string())
                .collect();

            // Only public functions conventionally receive @doc attribution.
            let doc = if !func.is_private() {
                doc_iter.next().map(|s| s.to_string())
            } else {
                None
            };

            (func.name.clone(), param_names, doc)
        })
        .collect()
}

/// Render markdown-style documentation for all modules in the AST.
fn render_docs(modules: &[Module]) -> String {
    let mut out = String::new();

    for module in modules {
        // Module header
        out.push_str(&format!("# {}\n", module.name));

        // Module-level documentation
        if let Some(doc) = module_doc(module) {
            out.push('\n');
            out.push_str(doc);
            out.push('\n');
        }

        // Functions section
        let funcs = function_docs(module);
        if !funcs.is_empty() {
            out.push_str("\n## Functions\n");

            for (name, params, doc) in funcs {
                let signature = if params.is_empty() {
                    format!("{}()", name)
                } else {
                    format!("{}({})", name, params.join(", "))
                };

                out.push('\n');
                out.push_str(&format!("### {}\n", signature));

                if let Some(doc_str) = doc {
                    out.push('\n');
                    out.push_str(&doc_str);
                    out.push('\n');
                }
            }
        }
    }

    out
}

/// Entry point for `tonic docs <path>`.
pub fn handle_docs(args: Vec<String>) -> i32 {
    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic docs --help` for usage",
        )
        .emit();
    }

    let source_path = &args[0];

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

    let output = render_docs(&ast.modules);
    print!("{output}");

    EXIT_OK
}
