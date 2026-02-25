use crate::parser::{Ast, Expr, ModuleForm, Pattern};
use crate::resolver_diag::ResolverError;
use std::collections::{HashMap, HashSet};

pub fn resolve_ast(ast: &Ast) -> Result<(), ResolverError> {
    ensure_no_duplicate_modules(ast)?;

    let module_graph = ModuleGraph::from_ast(ast);

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
                resolve_expr(guard, &context)?;
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

#[derive(Debug, Default)]
struct ModuleGraph {
    modules: HashMap<String, HashMap<String, FunctionVisibility>>,
    structs: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct FunctionVisibility {
    public: bool,
    private: bool,
}

enum CallResolution {
    Found,
    Missing,
    Private,
}

impl ModuleGraph {
    fn from_ast(ast: &Ast) -> Self {
        let mut modules: HashMap<String, HashMap<String, FunctionVisibility>> = HashMap::new();
        let mut structs: HashMap<String, HashSet<String>> = HashMap::new();

        for module in &ast.modules {
            let symbols = modules.entry(module.name.clone()).or_default();
            for function in &module.functions {
                let visibility = symbols.entry(function.name.clone()).or_default();
                if function.is_private() {
                    visibility.private = true;
                } else {
                    visibility.public = true;
                }
            }

            if let Some(fields) = module.forms.iter().find_map(|form| {
                if let ModuleForm::Defstruct { fields } = form {
                    Some(
                        fields
                            .iter()
                            .map(|field| field.name.clone())
                            .collect::<HashSet<_>>(),
                    )
                } else {
                    None
                }
            }) {
                structs.insert(module.name.clone(), fields);
            }
        }

        Self { modules, structs }
    }

    fn resolve_call_target(&self, current_module: &str, callee: &str) -> CallResolution {
        if is_builtin_call_target(callee) {
            return CallResolution::Found;
        }

        if let Some((module_name, function_name)) = callee.split_once('.') {
            let Some(module_symbols) = self.modules.get(module_name) else {
                return CallResolution::Missing;
            };

            let Some(symbol) = module_symbols.get(function_name) else {
                return CallResolution::Missing;
            };

            if symbol.public {
                return CallResolution::Found;
            }

            return if module_name == current_module && symbol.private {
                CallResolution::Found
            } else {
                CallResolution::Private
            };
        }

        self.modules
            .get(current_module)
            .and_then(|symbols| symbols.get(callee))
            .map(|_| CallResolution::Found)
            .unwrap_or(CallResolution::Missing)
    }

    fn has_struct_module(&self, module_name: &str) -> bool {
        self.structs.contains_key(module_name)
    }

    fn struct_has_field(&self, module_name: &str, field: &str) -> bool {
        self.structs
            .get(module_name)
            .is_some_and(|fields| fields.contains(field))
    }
}

fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err" | "tuple" | "list" | "map" | "keyword" | "protocol_dispatch" | "host_call"
    )
}

struct ResolveContext<'a> {
    module_name: &'a str,
    function_name: &'a str,
    module_graph: &'a ModuleGraph,
}

fn resolve_expr(expr: &Expr, context: &ResolveContext<'_>) -> Result<(), ResolverError> {
    match expr {
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Nil { .. }
        | Expr::String { .. } => Ok(()),
        Expr::InterpolatedString { segments, .. } => {
            for segment in segments {
                if let crate::parser::InterpolationSegment::Expr { expr } = segment {
                    resolve_expr(expr, context)?;
                }
            }
            Ok(())
        }
        Expr::Tuple { items, .. } | Expr::List { items, .. } => {
            for item in items {
                resolve_expr(item, context)?;
            }
            Ok(())
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                resolve_expr(entry.key(), context)?;
                resolve_expr(entry.value(), context)?;
            }
            Ok(())
        }
        Expr::Struct {
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
                if !context.module_graph.struct_has_field(module, &entry.key) {
                    return Err(ResolverError::unknown_struct_field(
                        &entry.key,
                        module,
                        context.module_name,
                        context.function_name,
                    ));
                }
                resolve_expr(&entry.value, context)?;
            }
            Ok(())
        }
        Expr::Keyword { entries, .. } => {
            for entry in entries {
                resolve_expr(&entry.value, context)?;
            }
            Ok(())
        }
        Expr::MapUpdate { base, updates, .. } => {
            resolve_expr(base, context)?;
            for entry in updates {
                resolve_expr(&entry.value, context)?;
            }
            Ok(())
        }
        Expr::StructUpdate {
            module,
            base,
            updates,
            ..
        } => {
            if !context.module_graph.has_struct_module(module) {
                return Err(ResolverError::undefined_struct_module(
                    module,
                    context.module_name,
                    context.function_name,
                ));
            }

            resolve_expr(base, context)?;
            for entry in updates {
                if !context.module_graph.struct_has_field(module, &entry.key) {
                    return Err(ResolverError::unknown_struct_field(
                        &entry.key,
                        module,
                        context.module_name,
                        context.function_name,
                    ));
                }
                resolve_expr(&entry.value, context)?;
            }
            Ok(())
        }
        Expr::FieldAccess { base, .. } => resolve_expr(base, context),
        Expr::IndexAccess { base, index, .. } => {
            resolve_expr(base, context)?;
            resolve_expr(index, context)
        }
        Expr::Call { callee, args, .. } => {
            match context
                .module_graph
                .resolve_call_target(context.module_name, callee)
            {
                CallResolution::Found => {}
                CallResolution::Missing => {
                    return Err(ResolverError::undefined_symbol(
                        callee,
                        context.module_name,
                        context.function_name,
                    ));
                }
                CallResolution::Private => {
                    return Err(ResolverError::private_function(
                        callee,
                        context.module_name,
                        context.function_name,
                    ));
                }
            }

            for arg in args {
                resolve_expr(arg, context)?;
            }

            Ok(())
        }
        Expr::Fn { body, .. } => resolve_expr(body, context),
        Expr::Invoke { callee, args, .. } => {
            resolve_expr(callee, context)?;
            for arg in args {
                resolve_expr(arg, context)?;
            }
            Ok(())
        }
        Expr::Question { value, .. } | Expr::Unary { value, .. } => resolve_expr(value, context),
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            resolve_expr(left, context)?;
            resolve_expr(right, context)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            resolve_expr(subject, context)?;

            for branch in branches {
                resolve_pattern(branch.head(), context)?;
                if let Some(guard) = branch.guard() {
                    resolve_expr(guard, context)?;
                }
                resolve_expr(branch.body(), context)?;
            }

            Ok(())
        }
        Expr::For {
            generators,
            into,
            body,
            ..
        } => {
            for (pattern, generator) in generators {
                resolve_pattern(pattern, context)?;
                resolve_expr(generator, context)?;
            }
            if let Some(into_expr) = into {
                resolve_expr(into_expr, context)?;
            }
            resolve_expr(body, context)
        }
        Expr::Group { inner, .. } => resolve_expr(inner, context),
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            resolve_expr(body, context)?;
            for branch in rescue {
                if let Some(guard) = branch.guard() {
                    resolve_expr(guard, context)?;
                }
                resolve_expr(branch.body(), context)?;
            }
            for branch in catch {
                if let Some(guard) = branch.guard() {
                    resolve_expr(guard, context)?;
                }
                resolve_expr(branch.body(), context)?;
            }
            if let Some(after) = after {
                resolve_expr(after, context)?;
            }
            Ok(())
        }
        Expr::Raise { error, .. } => resolve_expr(error, context),
        Expr::Variable { .. } | Expr::Atom { .. } => Ok(()),
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
mod tests {
    use super::resolve_ast;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;
    use crate::resolver_diag::ResolverDiagnosticCode;

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
    fn resolve_ast_accepts_builtin_result_constructors() {
        let source = "defmodule Demo do\n  def run() do\n    ok(1)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        resolve_ast(&ast).expect("resolver should accept result constructor builtins");
    }

    #[test]
    fn resolve_ast_accepts_builtin_collection_constructors() {
        let source =
            "defmodule Demo do\n  def run() do\n    tuple(map(1, 2), keyword(3, 4))\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        resolve_ast(&ast).expect("resolver should accept collection constructor builtins");
    }

    #[test]
    fn resolve_ast_accepts_builtin_protocol_dispatch() {
        let source = "defmodule Demo do\n  def run() do\n    tuple(protocol_dispatch(tuple(1, 2)), protocol_dispatch(map(3, 4)))\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        resolve_ast(&ast).expect("resolver should accept protocol dispatch builtin calls");
    }

    #[test]
    fn resolve_ast_reports_undefined_symbol_with_code() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
        let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject undefined calls");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedSymbol);
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

    #[test]
    fn resolve_ast_accepts_local_calls_to_private_functions() {
        let source = "defmodule Demo do\n  defp hidden() do\n    1\n  end\n\n  def run() do\n    hidden()\n  end\nend\n";
        let tokens =
            scan_tokens(source).expect("scanner should tokenize private local-call fixture");
        let ast = parse_ast(&tokens).expect("parser should build private local-call fixture ast");

        resolve_ast(&ast).expect("resolver should accept local calls to defp functions");
    }

    #[test]
    fn resolve_ast_rejects_cross_module_calls_to_private_functions() {
        let source = "defmodule Math do\n  defp hidden() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.hidden()\n  end\nend\n";
        let tokens =
            scan_tokens(source).expect("scanner should tokenize private visibility fixture");
        let ast = parse_ast(&tokens).expect("parser should build private visibility fixture ast");

        let error = resolve_ast(&ast)
            .expect_err("resolver should reject cross-module calls to private functions");

        assert_eq!(error.code(), ResolverDiagnosticCode::PrivateFunction);
        assert_eq!(
            error.to_string(),
            "[E1002] private function 'Math.hidden' cannot be called from Demo.run"
        );
    }

    #[test]
    fn resolve_ast_rejects_duplicate_module_definitions() {
        let source = "defmodule Shared do\n  def from_root() do\n    1\n  end\nend\n\ndefmodule Shared do\n  def from_dep() do\n    2\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize duplicate module fixture");
        let ast = parse_ast(&tokens).expect("parser should build duplicate module fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject duplicate modules");

        assert_eq!(error.code(), ResolverDiagnosticCode::DuplicateModule);
        assert_eq!(
            error.to_string(),
            "[E1003] duplicate module definition 'Shared'"
        );
    }

    #[test]
    fn resolve_ast_rejects_undefined_struct_module() {
        let source = "defmodule Demo do\n  def run() do\n    %Missing{name: 1}\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize struct module fixture");
        let ast = parse_ast(&tokens).expect("parser should build struct module fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject undefined struct modules");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedStructModule);
        assert_eq!(
            error.to_string(),
            "[E1004] undefined struct module 'Missing' in Demo.run"
        );
    }

    #[test]
    fn resolve_ast_rejects_unknown_struct_fields() {
        let source = "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run() do\n    %User{name: \"A\", agez: 42}\n  end\nend\n";
        let tokens =
            scan_tokens(source).expect("scanner should tokenize unknown struct field fixture");
        let ast = parse_ast(&tokens).expect("parser should build unknown struct field fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject unknown struct fields");

        assert_eq!(error.code(), ResolverDiagnosticCode::UnknownStructField);
        assert_eq!(
            error.to_string(),
            "[E1005] unknown struct field 'agez' for User in User.run"
        );
    }
}
