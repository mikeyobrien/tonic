use crate::guard_builtins;
use crate::parser::{Ast, Expr, ModuleForm, Pattern};
use crate::resolver_diag::ResolverError;
use std::collections::{HashMap, HashSet};

pub fn resolve_ast(ast: &Ast) -> Result<(), ResolverError> {
    ensure_no_duplicate_modules(ast)?;

    let module_graph = ModuleGraph::from_ast(ast)?;

    for module in &ast.modules {
        for function in &module.functions {
            let context = ResolveContext {
                module_name: &module.name,
                function_name: &function.name,
                module_graph: &module_graph,
            };

            for param in &function.params {
                resolve_pattern(param.pattern(), &context)?;
                if let Some(default) = param.default() {
                    resolve_expr(default, &context)?;
                }
            }

            if let Some(guard) = function.guard() {
                resolve_guard_expr(guard, &context)?;
            }

            resolve_expr(&function.body, &context)?;
        }
    }

    Ok(())
}

fn ensure_no_duplicate_modules(ast: &Ast) -> Result<(), ResolverError> {
    let mut seen = HashSet::new();

    for module in &ast.modules {
        if !seen.insert(module.name.as_str()) {
            return Err(ResolverError::duplicate_module(&module.name));
        }
    }

    Ok(())
}