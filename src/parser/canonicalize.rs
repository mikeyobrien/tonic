use super::*;
use std::collections::{HashMap, HashSet};

pub(super) fn collect_module_callable_signatures(
    modules: &[Module],
) -> HashMap<String, HashSet<(String, usize)>> {
    let mut callable = HashMap::new();

    for module in modules {
        let mut signatures = HashSet::new();

        for function in &module.functions {
            if function.is_private() {
                continue;
            }

            let max_arity = function.params.len();
            let default_count = function
                .params
                .iter()
                .rev()
                .take_while(|param| param.has_default())
                .count();
            let min_arity = max_arity.saturating_sub(default_count);

            for arity in min_arity..=max_arity {
                signatures.insert((function.name.clone(), arity));
            }
        }

        callable.insert(module.name.clone(), signatures);
    }

    callable
}

#[derive(Debug, Clone)]
struct ImportScope {
    module: String,
    only: Option<HashSet<(String, usize)>>,
    except: HashSet<(String, usize)>,
    exported_signatures: Option<HashSet<(String, usize)>>,
}

impl ImportScope {
    fn from_module_form(
        module: &str,
        only: &Option<Vec<ImportFunctionSpec>>,
        except: &Option<Vec<ImportFunctionSpec>>,
        callable_modules: &HashMap<String, HashSet<(String, usize)>>,
    ) -> Self {
        let only = only.as_ref().map(|entries| {
            entries
                .iter()
                .map(|entry| (entry.name.clone(), entry.arity))
                .collect::<HashSet<_>>()
        });

        let except = except
            .as_ref()
            .map(|entries| {
                entries
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.arity))
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        Self {
            module: module.to_string(),
            only,
            except,
            exported_signatures: callable_modules.get(module).cloned(),
        }
    }

    fn allows(&self, name: &str, arity: usize) -> bool {
        if self
            .exported_signatures
            .as_ref()
            .is_some_and(|signatures| !signatures.contains(&(name.to_string(), arity)))
        {
            return false;
        }
        if self.except.contains(&(name.to_string(), arity)) {
            return false;
        }

        if let Some(only) = &self.only {
            return only.contains(&(name.to_string(), arity));
        }

        true
    }
}

/// Shared context for canonicalization traversal — avoids passing 4 parameters to every
/// recursive call in `canonicalize_expr`.
struct CanonCtx<'a> {
    aliases: &'a HashMap<String, String>,
    imports: &'a [ImportScope],
    use_fallback_modules: &'a [String],
    local_functions: &'a HashSet<String>,
}

pub(super) fn canonicalize_module_call_targets(
    module: &mut Module,
    callable_modules: &HashMap<String, HashSet<(String, usize)>>,
) {
    // Scoped module-form semantics (parity task 04):
    // - `import Module` keeps existing behavior for unqualified call rewriting.
    // - `use Module` provides a limited compile-time effect by acting as an import fallback
    //   only when the module has no explicit imports.
    // - Full Elixir `__using__/1` macro expansion is intentionally deferred.
    let aliases = module
        .forms
        .iter()
        .filter_map(|form| match form {
            ModuleForm::Alias { module, as_name } => Some((as_name.clone(), module.clone())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let mut imports = Vec::new();
    let mut use_fallback_modules = Vec::new();
    for form in &module.forms {
        match form {
            ModuleForm::Import {
                module,
                only,
                except,
            } => {
                imports.push(ImportScope::from_module_form(
                    module,
                    only,
                    except,
                    callable_modules,
                ));
            }
            ModuleForm::Use { module } => {
                if !use_fallback_modules.contains(module) {
                    use_fallback_modules.push(module.clone());
                }
            }
            _ => {}
        }
    }

    let local_functions = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();

    let ctx = CanonCtx {
        aliases: &aliases,
        imports: &imports,
        use_fallback_modules: &use_fallback_modules,
        local_functions: &local_functions,
    };

    for form in &mut module.forms {
        match form {
            ModuleForm::Defstruct { fields } => {
                for field in fields {
                    canonicalize_expr(&mut field.default, &ctx);
                }
            }
            ModuleForm::Defimpl { functions, .. } => {
                for function in functions {
                    for param in &mut function.params {
                        if let Some(default) = param.default_mut() {
                            canonicalize_expr(default, &ctx);
                        }
                    }
                    if let Some(guard) = &mut function.guard {
                        canonicalize_expr(guard, &ctx);
                    }
                    canonicalize_expr(&mut function.body, &ctx);
                }
            }
            _ => {}
        }
    }

    for function in &mut module.functions {
        for param in &mut function.params {
            if let Some(default) = param.default_mut() {
                canonicalize_expr(default, &ctx);
            }
        }
        if let Some(guard) = &mut function.guard {
            canonicalize_expr(guard, &ctx);
        }
        canonicalize_expr(&mut function.body, &ctx);
    }
}

fn canonicalize_expr(expr: &mut Expr, ctx: &CanonCtx<'_>) {
    match expr {
        Expr::Tuple { items, .. } | Expr::List { items, .. } | Expr::Bitstring { items, .. } => {
            for item in items {
                canonicalize_expr(item, ctx);
            }
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                canonicalize_expr(&mut entry.key, ctx);
                canonicalize_expr(&mut entry.value, ctx);
            }
        }
        Expr::Struct { entries, .. } => {
            for entry in entries {
                canonicalize_expr(&mut entry.value, ctx);
            }
        }
        Expr::Keyword { entries, .. } => {
            for entry in entries {
                canonicalize_expr(&mut entry.value, ctx);
            }
        }
        Expr::MapUpdate { base, updates, .. } | Expr::StructUpdate { base, updates, .. } => {
            canonicalize_expr(base, ctx);
            for entry in updates {
                canonicalize_expr(&mut entry.value, ctx);
            }
        }
        Expr::FieldAccess { base, .. } => {
            canonicalize_expr(base, ctx);
        }
        Expr::IndexAccess { base, index, .. } => {
            canonicalize_expr(base, ctx);
            canonicalize_expr(index, ctx);
        }
        Expr::Call { callee, args, .. } => {
            let arity = args.len();
            for arg in args.iter_mut() {
                canonicalize_expr(arg, ctx);
            }
            canonicalize_call_target(callee, arity, ctx);
        }
        Expr::Fn { body, .. } => {
            canonicalize_expr(body, ctx);
        }
        Expr::Invoke { callee, args, .. } => {
            canonicalize_expr(callee, ctx);
            for arg in args {
                canonicalize_expr(arg, ctx);
            }
        }
        Expr::Question { value, .. }
        | Expr::Group { inner: value, .. }
        | Expr::Unary { value, .. } => {
            canonicalize_expr(value, ctx);
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            canonicalize_expr(left, ctx);
            canonicalize_expr(right, ctx);
        }
        Expr::Case {
            subject, branches, ..
        } => {
            canonicalize_expr(subject, ctx);
            for branch in branches {
                if let Some(guard) = branch.guard_mut() {
                    canonicalize_expr(guard, ctx);
                }
                canonicalize_expr(branch.body_mut(), ctx);
            }
        }
        Expr::For {
            generators,
            into,
            reduce,
            body,
            ..
        } => {
            for generator in generators {
                canonicalize_expr(generator.source_mut(), ctx);
                if let Some(guard) = generator.guard_mut() {
                    canonicalize_expr(guard, ctx);
                }
            }
            if let Some(into_expr) = into {
                canonicalize_expr(into_expr, ctx);
            }
            if let Some(reduce_expr) = reduce {
                canonicalize_expr(reduce_expr, ctx);
            }
            canonicalize_expr(body, ctx);
        }
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            canonicalize_expr(body, ctx);
            for branch in rescue {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr(guard, ctx);
                }
                canonicalize_expr(&mut branch.body, ctx);
            }
            for branch in catch {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr(guard, ctx);
                }
                canonicalize_expr(&mut branch.body, ctx);
            }
            if let Some(after) = after {
                canonicalize_expr(after, ctx);
            }
        }
        Expr::Raise { error, .. } => {
            canonicalize_expr(error, ctx);
        }
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Nil { .. }
        | Expr::String { .. }
        | Expr::Variable { .. }
        | Expr::Atom { .. } => {}
        Expr::InterpolatedString { segments, .. } => {
            for segment in segments {
                if let InterpolationSegment::Expr { expr } = segment {
                    canonicalize_expr(expr, ctx);
                }
            }
        }
    }
}

fn canonicalize_call_target(callee: &mut String, arity: usize, ctx: &CanonCtx<'_>) {
    if let Some((alias_name, function_name)) = callee.split_once('.') {
        if let Some(module_name) = ctx.aliases.get(alias_name) {
            *callee = format!("{module_name}.{function_name}");
        }
        return;
    }

    if ctx.local_functions.contains(callee.as_str()) || is_builtin_call_target(callee) {
        return;
    }

    let mut import_matches = ctx
        .imports
        .iter()
        .filter(|scope| scope.allows(callee, arity))
        .map(|scope| scope.module.as_str())
        .collect::<Vec<_>>();
    import_matches.sort_unstable();
    import_matches.dedup();

    if import_matches.len() == 1 {
        *callee = format!("{}.{}", import_matches[0], callee);
    } else if ctx.imports.is_empty() && ctx.use_fallback_modules.len() == 1 {
        *callee = format!("{}.{}", ctx.use_fallback_modules[0], callee);
    }
}
