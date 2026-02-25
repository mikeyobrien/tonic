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

#[derive(Debug, Default)]
struct ModuleGraph {
    modules: HashMap<String, HashMap<String, FunctionVisibility>>,
    structs: HashMap<String, HashSet<String>>,
    protocols: HashMap<String, ProtocolDefinition>,
    imports: HashMap<String, Vec<ImportScope>>,
}

#[derive(Debug, Default)]
struct ProtocolDefinition {
    functions: HashMap<String, usize>,
    impl_targets: HashSet<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct FunctionVisibility {
    public: bool,
    private: bool,
}

#[derive(Debug, Clone)]
struct ImportScope {
    module: String,
    only: Option<HashSet<(String, usize)>>,
    except: HashSet<(String, usize)>,
}

impl ImportScope {
    fn from_form(
        module: &str,
        only: &Option<Vec<crate::parser::ImportFunctionSpec>>,
        except: &Option<Vec<crate::parser::ImportFunctionSpec>>,
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
        }
    }

    fn allows(&self, name: &str, arity: usize) -> bool {
        if self.except.contains(&(name.to_string(), arity)) {
            return false;
        }

        if let Some(only) = &self.only {
            return only.contains(&(name.to_string(), arity));
        }

        true
    }
}

enum CallResolution {
    Found,
    Missing,
    Private,
}

impl ModuleGraph {
    fn from_ast(ast: &Ast) -> Result<Self, ResolverError> {
        let mut modules: HashMap<String, HashMap<String, FunctionVisibility>> = HashMap::new();
        let mut structs: HashMap<String, HashSet<String>> = HashMap::new();
        let mut protocols: HashMap<String, ProtocolDefinition> = HashMap::new();
        let mut imports: HashMap<String, Vec<ImportScope>> = HashMap::new();

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

            for form in &module.forms {
                if let ModuleForm::Import {
                    module: imported_module,
                    only,
                    except,
                } = form
                {
                    imports
                        .entry(module.name.clone())
                        .or_default()
                        .push(ImportScope::from_form(imported_module, only, except));
                }
            }
        }

        for module in &ast.modules {
            for form in &module.forms {
                let ModuleForm::Defprotocol { name, functions } = form else {
                    continue;
                };

                if protocols.contains_key(name) {
                    return Err(ResolverError::duplicate_protocol(name));
                }

                let mut signatures = HashMap::new();
                for function in functions {
                    let arity = function.params.len();
                    if signatures.insert(function.name.clone(), arity).is_some() {
                        return Err(ResolverError::duplicate_protocol_function(
                            name,
                            &function.name,
                            arity,
                        ));
                    }

                    modules
                        .entry(name.clone())
                        .or_default()
                        .entry(function.name.clone())
                        .or_default()
                        .public = true;
                }

                protocols.insert(
                    name.clone(),
                    ProtocolDefinition {
                        functions: signatures,
                        impl_targets: HashSet::new(),
                    },
                );
            }
        }

        // Scoped module-form semantics (parity task 04):
        // - `require Module` declares a compile-time dependency and must target a defined module.
        // - `use Module` must also target a defined module; its compile-time call-rewrite effect
        //   is implemented in parser canonicalization as a fallback import.
        // - Full Elixir macro expansion via `__using__/1` is intentionally deferred.
        for module in &ast.modules {
            for form in &module.forms {
                match form {
                    ModuleForm::Require {
                        module: required_module,
                    } => {
                        if !modules.contains_key(required_module) {
                            return Err(ResolverError::undefined_required_module(
                                required_module,
                                &module.name,
                            ));
                        }
                    }
                    ModuleForm::Use {
                        module: used_module,
                    } => {
                        if !modules.contains_key(used_module) {
                            return Err(ResolverError::undefined_use_module(
                                used_module,
                                &module.name,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        for module in &ast.modules {
            for form in &module.forms {
                let ModuleForm::Defimpl {
                    protocol,
                    target,
                    functions,
                } = form
                else {
                    continue;
                };

                let Some(protocol_definition) = protocols.get_mut(protocol) else {
                    return Err(ResolverError::unknown_protocol(protocol, target));
                };

                if !protocol_definition.impl_targets.insert(target.clone()) {
                    return Err(ResolverError::duplicate_protocol_impl(protocol, target));
                }

                let mut implemented = HashMap::new();
                for function in functions {
                    let arity = function.params.len();
                    if implemented.insert(function.name.clone(), arity).is_some() {
                        return Err(ResolverError::invalid_protocol_impl(
                            protocol,
                            target,
                            &function.name,
                            arity,
                            "is declared more than once",
                        ));
                    }

                    if function.params.iter().any(|param| param.has_default()) {
                        return Err(ResolverError::invalid_protocol_impl(
                            protocol,
                            target,
                            &function.name,
                            arity,
                            "must not use default parameters",
                        ));
                    }

                    let Some(expected_arity) = protocol_definition.functions.get(&function.name)
                    else {
                        return Err(ResolverError::invalid_protocol_impl(
                            protocol,
                            target,
                            &function.name,
                            arity,
                            "is not declared by the protocol",
                        ));
                    };

                    if *expected_arity != arity {
                        return Err(ResolverError::invalid_protocol_impl(
                            protocol,
                            target,
                            &function.name,
                            arity,
                            &format!("has arity mismatch (expected {expected_arity})"),
                        ));
                    }
                }

                for (name, arity) in &protocol_definition.functions {
                    if implemented.get(name).copied() != Some(*arity) {
                        return Err(ResolverError::invalid_protocol_impl(
                            protocol,
                            target,
                            name,
                            *arity,
                            "is missing",
                        ));
                    }
                }
            }
        }

        Ok(Self {
            modules,
            structs,
            protocols,
            imports,
        })
    }

    fn resolve_call_target(
        &self,
        current_module: &str,
        callee: &str,
        arity: Option<usize>,
    ) -> CallResolution {
        if is_builtin_call_target(callee) {
            return CallResolution::Found;
        }

        if let Some((module_name, function_name)) = callee.split_once('.') {
            if let Some(protocol) = self.protocols.get(module_name) {
                return if protocol.functions.contains_key(function_name) {
                    CallResolution::Found
                } else {
                    CallResolution::Missing
                };
            }

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

        if self
            .modules
            .get(current_module)
            .and_then(|symbols| symbols.get(callee))
            .is_some()
        {
            return CallResolution::Found;
        }

        if let Some(arity) = arity {
            let mut candidates = self
                .imports
                .get(current_module)
                .into_iter()
                .flatten()
                .filter_map(|scope| {
                    let visibility = self
                        .modules
                        .get(&scope.module)
                        .and_then(|symbols| symbols.get(callee))?;
                    if !visibility.public || !scope.allows(callee, arity) {
                        return None;
                    }
                    Some(scope.module.as_str())
                })
                .collect::<Vec<_>>();
            candidates.sort_unstable();
            candidates.dedup();

            if candidates.len() == 1 {
                return CallResolution::Found;
            }
        }

        CallResolution::Missing
    }

    fn import_filter_diagnostic(
        &self,
        current_module: &str,
        function_name: &str,
        arity: usize,
    ) -> Option<ResolverError> {
        let scopes = self.imports.get(current_module)?;

        let mut modules_with_symbol = Vec::new();
        let mut allowed_modules = Vec::new();

        for scope in scopes {
            let Some(symbols) = self.modules.get(&scope.module) else {
                continue;
            };

            let Some(visibility) = symbols.get(function_name) else {
                continue;
            };

            if !visibility.public {
                continue;
            }

            modules_with_symbol.push(scope.module.clone());

            if scope.allows(function_name, arity) {
                allowed_modules.push(scope.module.clone());
            }
        }

        modules_with_symbol.sort_unstable();
        modules_with_symbol.dedup();
        allowed_modules.sort_unstable();
        allowed_modules.dedup();

        if allowed_modules.len() > 1 {
            return Some(ResolverError::ambiguous_import_call(
                function_name,
                arity,
                current_module,
                &allowed_modules,
            ));
        }

        if allowed_modules.is_empty() && !modules_with_symbol.is_empty() {
            return Some(ResolverError::import_filter_excludes_call(
                function_name,
                arity,
                current_module,
                &modules_with_symbol,
            ));
        }

        None
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
        Expr::Tuple { items, .. } | Expr::List { items, .. } => {
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
            ..
        } => {
            if !context.module_graph.has_struct_module(module) {
                return Err(ResolverError::undefined_struct_module(
                    module,
                    context.module_name,
                    context.function_name,
                ));
            }

            resolve_expr_with_guard_context(base, context, in_guard_context)?;
            for entry in updates {
                if !context.module_graph.struct_has_field(module, &entry.key) {
                    return Err(ResolverError::unknown_struct_field(
                        &entry.key,
                        module,
                        context.module_name,
                        context.function_name,
                    ));
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
        Expr::Call { callee, args, .. } => {
            if guard_builtins::is_guard_builtin(callee) {
                if !in_guard_context {
                    return Err(ResolverError::guard_builtin_outside_guard(
                        callee,
                        guard_builtins::guard_builtin_arity(callee).unwrap_or(args.len()),
                        context.module_name,
                        context.function_name,
                    ));
                }
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
                            ) {
                                return Err(error);
                            }
                        }

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
            body,
            ..
        } => {
            for (pattern, generator) in generators {
                resolve_pattern(pattern, context)?;
                resolve_expr_with_guard_context(generator, context, in_guard_context)?;
            }
            if let Some(into_expr) = into {
                resolve_expr_with_guard_context(into_expr, context, in_guard_context)?;
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
    fn resolve_ast_accepts_use_with_defined_module_target() {
        let source = "defmodule Feature do\n  def helper() do\n    41\n  end\nend\n\ndefmodule Demo do\n  use Feature\n\n  def run() do\n    helper()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize use fixture");
        let ast = parse_ast(&tokens).expect("parser should build use fixture ast");

        resolve_ast(&ast).expect("resolver should accept use with a defined module target");
    }

    #[test]
    fn resolve_ast_accepts_import_only_and_except_filters() {
        let source = "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n\n  def run() do\n    add(20, 22)\n  end\nend\n\ndefmodule SafeDemo do\n  import Math, except: [unsafe: 1]\n\n  def run() do\n    add(2, 3)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize import filter fixture");
        let ast = parse_ast(&tokens).expect("parser should build import filter fixture ast");

        resolve_ast(&ast).expect("resolver should accept valid import only/except filters");
    }

    #[test]
    fn resolve_ast_rejects_calls_excluded_by_import_filters() {
        let source = "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [other: 1]\n\n  def run() do\n    helper(1)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize filtered import fixture");
        let ast = parse_ast(&tokens).expect("parser should build filtered import fixture ast");

        let error =
            resolve_ast(&ast).expect_err("resolver should reject calls excluded by import filters");

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::ImportFilterExcludesCall
        );
        assert_eq!(
            error.to_string(),
            "[E1013] import filters exclude call 'helper/1' in Demo; imported modules with this symbol: Math"
        );
    }

    #[test]
    fn resolve_ast_rejects_ambiguous_calls_after_import_filtering() {
        let source = "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Helpers do\n  def helper(value) do\n    value + 1\n  end\nend\n\ndefmodule Demo do\n  import Math, except: [other: 1]\n  import Helpers, only: [helper: 1]\n\n  def run() do\n    helper(1)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize ambiguous import fixture");
        let ast = parse_ast(&tokens).expect("parser should build ambiguous import fixture ast");

        let error = resolve_ast(&ast)
            .expect_err("resolver should reject ambiguous calls after import filtering");

        assert_eq!(error.code(), ResolverDiagnosticCode::AmbiguousImportCall);
        assert_eq!(
            error.to_string(),
            "[E1014] ambiguous imported call 'helper/1' in Demo; matches: Helpers, Math"
        );
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
    fn resolve_ast_accepts_guard_builtins_in_when_clauses() {
        let source = "defmodule Demo do\n  def choose(value) when is_integer(value) do\n    value\n  end\n\n  def choose(value) do\n    value\n  end\n\n  def run() do\n    case 1 do\n      current when is_number(current) -> choose(current)\n      _ -> 0\n    end\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize guard builtin fixture");
        let ast = parse_ast(&tokens).expect("parser should build guard builtin fixture ast");

        resolve_ast(&ast).expect("resolver should accept guard builtins in when clauses");
    }

    #[test]
    fn resolve_ast_rejects_guard_builtin_outside_guard_with_code() {
        let source = "defmodule Demo do\n  def run() do\n    is_integer(1)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize guard misuse fixture");
        let ast = parse_ast(&tokens).expect("parser should build guard misuse fixture ast");

        let error =
            resolve_ast(&ast).expect_err("resolver should reject guard builtin call outside guard");

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::GuardBuiltinOutsideGuard
        );
        assert_eq!(
            error.to_string(),
            "[E1015] guard builtin 'is_integer/1' is only allowed in guard expressions (when) in Demo.run"
        );
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
    fn resolve_ast_rejects_undefined_required_module() {
        let source = "defmodule Demo do\n  require Missing\n\n  def run() do\n    1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize require fixture");
        let ast = parse_ast(&tokens).expect("parser should build require fixture ast");

        let error =
            resolve_ast(&ast).expect_err("resolver should reject undefined required modules");

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::UndefinedRequiredModule
        );
        assert_eq!(
            error.to_string(),
            "[E1011] required module 'Missing' is not defined for Demo; add the module or remove require"
        );
    }

    #[test]
    fn resolve_ast_rejects_undefined_used_module() {
        let source = "defmodule Demo do\n  use Missing\n\n  def run() do\n    1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize use fixture");
        let ast = parse_ast(&tokens).expect("parser should build use fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject undefined used modules");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedUseModule);
        assert_eq!(
            error.to_string(),
            "[E1012] used module 'Missing' is not defined for Demo; add the module or remove use"
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

    #[test]
    fn resolve_ast_accepts_defprotocol_and_defimpl_forms() {
        let source = "defmodule User do\n  defstruct age: 0\nend\n\ndefmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  defimpl Size, for: User do\n    def size(user) do\n      user.age\n    end\n  end\n\n  def run(user) do\n    tuple(Size.size(tuple(1, 2)), Size.size(user))\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize protocol fixture");
        let ast = parse_ast(&tokens).expect("parser should build protocol fixture ast");

        resolve_ast(&ast).expect("resolver should accept protocol declaration and impl forms");
    }

    #[test]
    fn resolve_ast_rejects_unknown_protocol_impl_target() {
        let source = "defmodule Demo do\n  defimpl Missing, for: Tuple do\n    def size(_value) do\n      1\n    end\n  end\n\n  def run() do\n    0\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize unknown protocol fixture");
        let ast = parse_ast(&tokens).expect("parser should build unknown protocol fixture ast");

        let error = resolve_ast(&ast).expect_err("resolver should reject unknown defimpl protocol");

        assert_eq!(error.code(), ResolverDiagnosticCode::UnknownProtocol);
        assert_eq!(
            error.to_string(),
            "[E1008] unknown protocol 'Missing' for defimpl target 'Tuple'"
        );
    }

    #[test]
    fn resolve_ast_rejects_protocol_impl_arity_mismatch() {
        let source = "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value, _extra) do\n      2\n    end\n  end\n\n  def run() do\n    0\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize arity mismatch fixture");
        let ast = parse_ast(&tokens).expect("parser should build arity mismatch fixture ast");

        let error =
            resolve_ast(&ast).expect_err("resolver should reject protocol impl arity mismatch");

        assert_eq!(error.code(), ResolverDiagnosticCode::InvalidProtocolImpl);
        assert_eq!(
            error.to_string(),
            "[E1010] invalid defimpl for protocol 'Size' target 'Tuple': size/2 has arity mismatch (expected 1)"
        );
    }
}
