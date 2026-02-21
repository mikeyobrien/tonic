use crate::parser::{Ast, Expr};
use std::collections::{HashMap, HashSet};
use std::fmt;

const E1001_UNDEFINED_SYMBOL: &str = "E1001";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolverError {
    code: &'static str,
    message: String,
}

impl ResolverError {
    fn undefined_symbol(symbol: &str, module: &str, function: &str) -> Self {
        Self {
            code: E1001_UNDEFINED_SYMBOL,
            message: format!("undefined symbol '{symbol}' in {module}.{function}"),
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ResolverError {}

pub fn resolve_ast(ast: &Ast) -> Result<(), ResolverError> {
    let module_graph = ModuleGraph::from_ast(ast);

    for module in &ast.modules {
        for function in &module.functions {
            let context = ResolveContext {
                module_name: &module.name,
                function_name: &function.name,
                module_graph: &module_graph,
            };

            resolve_expr(&function.body, &context)?;
        }
    }

    Ok(())
}

struct ModuleGraph {
    modules: HashMap<String, HashSet<String>>,
}

impl ModuleGraph {
    fn from_ast(ast: &Ast) -> Self {
        let modules = ast
            .modules
            .iter()
            .map(|module| {
                let functions = module
                    .functions
                    .iter()
                    .map(|function| function.name.clone())
                    .collect();

                (module.name.clone(), functions)
            })
            .collect();

        Self { modules }
    }

    fn contains_call_target(&self, current_module: &str, callee: &str) -> bool {
        if let Some((module_name, function_name)) = callee.split_once('.') {
            return self
                .modules
                .get(module_name)
                .is_some_and(|symbols| symbols.contains(function_name));
        }

        self.modules
            .get(current_module)
            .is_some_and(|symbols| symbols.contains(callee))
    }
}

struct ResolveContext<'a> {
    module_name: &'a str,
    function_name: &'a str,
    module_graph: &'a ModuleGraph,
}

fn resolve_expr(expr: &Expr, context: &ResolveContext<'_>) -> Result<(), ResolverError> {
    match expr {
        Expr::Int { .. } => Ok(()),
        Expr::Call { callee, args, .. } => {
            if !context
                .module_graph
                .contains_call_target(context.module_name, callee)
            {
                return Err(ResolverError::undefined_symbol(
                    callee,
                    context.module_name,
                    context.function_name,
                ));
            }

            for arg in args {
                resolve_expr(arg, context)?;
            }

            Ok(())
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            resolve_expr(left, context)?;
            resolve_expr(right, context)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            resolve_expr(subject, context)?;

            for branch in branches {
                resolve_expr(branch.body(), context)?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_ast;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;

    #[test]
    fn resolve_ast_accepts_module_local_function_calls() {
        let source = "defmodule Demo do\n  def run() do\n    helper()\n  end\n\n  def helper() do\n    1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        resolve_ast(&ast).expect("resolver should accept local module references");
    }

    #[test]
    fn resolve_ast_accepts_module_qualified_function_calls() {
        let source = "defmodule Math do\n  def helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        resolve_ast(&ast).expect("resolver should accept module-qualified references");
    }

    #[test]
    fn resolve_ast_reports_undefined_symbol_with_code() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject undefined calls");

        assert_eq!(
            error.to_string(),
            "[E1001] undefined symbol 'missing' in Demo.run"
        );
    }

    #[test]
    fn resolve_ast_reports_missing_qualified_symbol_with_code() {
        let source = "defmodule Math do\n  def helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.unknown()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        let error =
            resolve_ast(&ast).expect_err("resolver should reject undefined module-qualified calls");

        assert_eq!(
            error.to_string(),
            "[E1001] undefined symbol 'Math.unknown' in Demo.run"
        );
    }
}
