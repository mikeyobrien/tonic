use crate::guard_builtins;
use crate::parser::{Ast, Expr, Pattern};
use crate::resolver_diag::ResolverError;
use std::collections::HashMap;

/// External module signatures known from prior REPL inputs or other contexts.
/// Maps module name → (function name → is_public).
pub type ExternalModules = HashMap<String, HashMap<String, bool>>;

#[path = "resolver_graph.rs"]
mod graph;
use graph::{ensure_no_duplicate_modules, CallResolution, ModuleGraph, UndefinedCallSuggestion};

pub fn resolve_ast(ast: &Ast) -> Result<(), ResolverError> {
    resolve_ast_with_externals(ast, &ExternalModules::new())
}

pub fn resolve_ast_with_externals(
    ast: &Ast,
    externals: &ExternalModules,
) -> Result<(), ResolverError> {
    ensure_no_duplicate_modules(ast)?;

    let mut module_graph = ModuleGraph::from_ast(ast)?;
    module_graph.merge_externals(externals);

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

struct ResolveContext<'a> {
    module_name: &'a str,
    function_name: &'a str,
    module_graph: &'a ModuleGraph,
}

fn resolve_expr(expr: &Expr, context: &ResolveContext<'_>) -> Result<(), ResolverError> {
    resolve_expr_with_guard_context(expr, context, false)
}

fn resolve_guard_expr(expr: &Expr, context: &ResolveContext<'_>) -> Result<(), ResolverError> {
    resolve_expr_with_guard_context(expr, context, true)
}

fn resolve_expr_with_guard_context(
    expr: &Expr,
    context: &ResolveContext<'_>,
    in_guard_context: bool,
) -> Result<(), ResolverError> {
    match expr {
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Nil { .. }
        | Expr::String { .. } => Ok(()),
        Expr::InterpolatedString { segments, .. } => {
            for segment in segments {
                if let crate::parser::InterpolationSegment::Expr { expr } = segment {
                    resolve_expr_with_guard_context(expr, context, in_guard_context)?;
                }
            }
            Ok(())
        }
        Expr::Tuple { items, .. } | Expr::List { items, .. } | Expr::Bitstring { items, .. } => {
            for item in items {
                resolve_expr_with_guard_context(item, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                resolve_expr_with_guard_context(entry.key(), context, in_guard_context)?;
                resolve_expr_with_guard_context(entry.value(), context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::Struct {
            module,
            entries,
            offset,
            ..
        } => {
            if !context.module_graph.has_struct_module(module) {
                return Err(ResolverError::undefined_struct_module(
                    module,
                    context.module_name,
                    context.function_name,
                )
                .with_offset(*offset));
            }

            for entry in entries {
                if !context.module_graph.struct_has_field(module, &entry.key) {
                    return Err(ResolverError::unknown_struct_field(
                        &entry.key,
                        module,
                        context.module_name,
                        context.function_name,
                    )
                    .with_offset(*offset));
                }
                resolve_expr_with_guard_context(&entry.value, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::Keyword { entries, .. } => {
            for entry in entries {
                resolve_expr_with_guard_context(&entry.value, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::MapUpdate { base, updates, .. } => {
            resolve_expr_with_guard_context(base, context, in_guard_context)?;
            for entry in updates {
                resolve_expr_with_guard_context(&entry.value, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::StructUpdate {
            module,
            base,
            updates,
            offset,
            ..
        } => {
            if !context.module_graph.has_struct_module(module) {
                return Err(ResolverError::undefined_struct_module(
                    module,
                    context.module_name,
                    context.function_name,
                )
                .with_offset(*offset));
            }

            resolve_expr_with_guard_context(base, context, in_guard_context)?;
            for entry in updates {
                if !context.module_graph.struct_has_field(module, &entry.key) {
                    return Err(ResolverError::unknown_struct_field(
                        &entry.key,
                        module,
                        context.module_name,
                        context.function_name,
                    )
                    .with_offset(*offset));
                }
                resolve_expr_with_guard_context(&entry.value, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::FieldAccess { base, .. } => {
            resolve_expr_with_guard_context(base, context, in_guard_context)
        }
        Expr::IndexAccess { base, index, .. } => {
            resolve_expr_with_guard_context(base, context, in_guard_context)?;
            resolve_expr_with_guard_context(index, context, in_guard_context)
        }
        Expr::Call {
            callee,
            args,
            offset,
            ..
        } => {
            if guard_builtins::is_guard_builtin(callee) {
                // Guard builtins are allowed everywhere — in guards AND
                // as regular boolean expressions (matching Elixir semantics).
            } else {
                match context.module_graph.resolve_call_target(
                    context.module_name,
                    callee,
                    Some(args.len()),
                ) {
                    CallResolution::Found => {}
                    CallResolution::Missing => {
                        if !callee.contains('.') {
                            if let Some(error) = context.module_graph.import_filter_diagnostic(
                                context.module_name,
                                callee,
                                args.len(),
                                *offset,
                            ) {
                                return Err(error);
                            }
                        }

                        let mut hint_parts = Vec::new();

                        if let Some(suggestion) = context.module_graph.undefined_call_suggestion(
                            context.module_name,
                            callee,
                            args.len(),
                        ) {
                            let hint = match suggestion {
                                UndefinedCallSuggestion::DidYouMean { target } => {
                                    ResolverError::did_you_mean_hint(&target)
                                }
                                UndefinedCallSuggestion::Imported { module, target } => {
                                    ResolverError::imported_did_you_mean_hint(&module, &target)
                                }
                                UndefinedCallSuggestion::Import {
                                    module,
                                    qualified_target,
                                    unqualified_target,
                                } => ResolverError::import_call_hint(
                                    &module,
                                    &qualified_target,
                                    &unqualified_target,
                                ),
                            };
                            hint_parts.push(hint);
                        }

                        if let Some((mod_name, _)) = callee.rsplit_once('.') {
                            if let Some(functions) =
                                context.module_graph.public_function_names(mod_name)
                            {
                                hint_parts.push(ResolverError::available_module_functions_hint(
                                    mod_name, &functions,
                                ));
                            }
                        }

                        let hint = (!hint_parts.is_empty()).then(|| hint_parts.join(""));
                        return Err(ResolverError::undefined_symbol_with_hint(
                            callee,
                            context.module_name,
                            context.function_name,
                            hint.as_deref(),
                        )
                        .with_offset(*offset));
                    }
                    CallResolution::Private => {
                        return Err(ResolverError::private_function(
                            callee,
                            context.module_name,
                            context.function_name,
                        )
                        .with_offset(*offset));
                    }
                }
            }

            for arg in args {
                resolve_expr_with_guard_context(arg, context, in_guard_context)?;
            }

            Ok(())
        }
        Expr::Fn { body, .. } => resolve_expr_with_guard_context(body, context, in_guard_context),
        Expr::Invoke { callee, args, .. } => {
            resolve_expr_with_guard_context(callee, context, in_guard_context)?;
            for arg in args {
                resolve_expr_with_guard_context(arg, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::Question { value, .. } | Expr::Unary { value, .. } => {
            resolve_expr_with_guard_context(value, context, in_guard_context)
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            resolve_expr_with_guard_context(left, context, in_guard_context)?;
            resolve_expr_with_guard_context(right, context, in_guard_context)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            resolve_expr_with_guard_context(subject, context, in_guard_context)?;

            for branch in branches {
                resolve_pattern(branch.head(), context)?;
                if let Some(guard) = branch.guard() {
                    resolve_guard_expr(guard, context)?;
                }
                resolve_expr_with_guard_context(branch.body(), context, in_guard_context)?;
            }

            Ok(())
        }
        Expr::For {
            generators,
            into,
            reduce,
            body,
            ..
        } => {
            for generator in generators {
                resolve_pattern(generator.pattern(), context)?;
                resolve_expr_with_guard_context(generator.source(), context, in_guard_context)?;
                if let Some(guard) = generator.guard() {
                    resolve_guard_expr(guard, context)?;
                }
            }
            if let Some(into_expr) = into {
                resolve_expr_with_guard_context(into_expr, context, in_guard_context)?;
            }
            if let Some(reduce_expr) = reduce {
                resolve_expr_with_guard_context(reduce_expr, context, in_guard_context)?;
            }
            resolve_expr_with_guard_context(body, context, in_guard_context)
        }
        Expr::Group { inner, .. } => {
            resolve_expr_with_guard_context(inner, context, in_guard_context)
        }
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            resolve_expr_with_guard_context(body, context, in_guard_context)?;
            for branch in rescue {
                if let Some(guard) = branch.guard() {
                    resolve_guard_expr(guard, context)?;
                }
                resolve_expr_with_guard_context(branch.body(), context, in_guard_context)?;
            }
            for branch in catch {
                if let Some(guard) = branch.guard() {
                    resolve_guard_expr(guard, context)?;
                }
                resolve_expr_with_guard_context(branch.body(), context, in_guard_context)?;
            }
            if let Some(after) = after {
                resolve_expr_with_guard_context(after, context, in_guard_context)?;
            }
            Ok(())
        }
        Expr::Raise { error, .. } => {
            resolve_expr_with_guard_context(error, context, in_guard_context)
        }
        Expr::Variable { .. } | Expr::Atom { .. } => Ok(()),
        Expr::Block { exprs, .. } => {
            for sub_expr in exprs {
                resolve_expr_with_guard_context(sub_expr, context, in_guard_context)?;
            }
            Ok(())
        }
    }
}

fn resolve_pattern(pattern: &Pattern, context: &ResolveContext<'_>) -> Result<(), ResolverError> {
    match pattern {
        Pattern::Atom { .. }
        | Pattern::Bind { .. }
        | Pattern::Pin { .. }
        | Pattern::Wildcard
        | Pattern::Integer { .. }
        | Pattern::Bool { .. }
        | Pattern::Nil
        | Pattern::String { .. } => Ok(()),
        Pattern::Tuple { items } => {
            for item in items {
                resolve_pattern(item, context)?;
            }
            Ok(())
        }
        Pattern::List { items, tail } => {
            for item in items {
                resolve_pattern(item, context)?;
            }
            if let Some(tail) = tail {
                resolve_pattern(tail, context)?;
            }
            Ok(())
        }
        Pattern::Map { entries } => {
            for entry in entries {
                resolve_pattern(entry.key(), context)?;
                resolve_pattern(entry.value(), context)?;
            }
            Ok(())
        }
        Pattern::Bitstring { items } => {
            for item in items {
                resolve_pattern(item, context)?;
            }
            Ok(())
        }
        Pattern::Struct {
            module, entries, ..
        } => {
            if !context.module_graph.has_struct_module(module) {
                return Err(ResolverError::undefined_struct_module(
                    module,
                    context.module_name,
                    context.function_name,
                ));
            }

            for entry in entries {
                if !context.module_graph.struct_has_field(module, entry.key()) {
                    return Err(ResolverError::unknown_struct_field(
                        entry.key(),
                        module,
                        context.module_name,
                        context.function_name,
                    ));
                }
                resolve_pattern(entry.value(), context)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
