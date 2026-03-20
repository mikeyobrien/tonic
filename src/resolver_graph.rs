use super::ExternalModules;
use crate::parser::{Ast, ModuleForm};
use crate::resolver_diag::ResolverError;
use std::collections::{HashMap, HashSet};

pub(super) fn ensure_no_duplicate_modules(ast: &Ast) -> Result<(), ResolverError> {
    let mut seen = HashSet::new();

    for module in &ast.modules {
        if !seen.insert(module.name.as_str()) {
            return Err(ResolverError::duplicate_module(&module.name));
        }
    }

    Ok(())
}

#[derive(Debug, Default)]

pub(super) struct ModuleGraph {
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

pub(super) enum CallResolution {
    Found,
    Missing,
    Private,
}

impl ModuleGraph {
    pub(super) fn from_ast(ast: &Ast) -> Result<Self, ResolverError> {
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

    pub(super) fn resolve_call_target(
        &self,
        current_module: &str,
        callee: &str,
        arity: Option<usize>,
    ) -> CallResolution {
        if is_builtin_call_target(callee) {
            return CallResolution::Found;
        }

        // Use rsplit_once to split on the LAST dot, so "Foo.Bar.greet" → module="Foo.Bar", fn="greet"
        if let Some((module_name, function_name)) = callee.rsplit_once('.') {
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

    pub(super) fn public_function_names(&self, module_name: &str) -> Option<Vec<String>> {
        let symbols = self.modules.get(module_name)?;
        let mut names: Vec<String> = symbols
            .iter()
            .filter(|(_, vis)| vis.public)
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        if names.is_empty() {
            None
        } else {
            Some(names)
        }
    }

    pub(super) fn import_filter_diagnostic(
        &self,
        current_module: &str,
        function_name: &str,
        arity: usize,
        offset: usize,
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
            return Some(
                ResolverError::ambiguous_import_call(
                    function_name,
                    arity,
                    current_module,
                    &allowed_modules,
                )
                .with_offset(offset),
            );
        }

        if allowed_modules.is_empty() && !modules_with_symbol.is_empty() {
            return Some(
                ResolverError::import_filter_excludes_call(
                    function_name,
                    arity,
                    current_module,
                    &modules_with_symbol,
                )
                .with_offset(offset),
            );
        }

        None
    }

    pub(super) fn merge_externals(&mut self, externals: &ExternalModules) {
        for (mod_name, functions) in externals {
            if !self.modules.contains_key(mod_name) {
                let vis_map: HashMap<String, FunctionVisibility> = functions
                    .iter()
                    .map(|(fn_name, &is_public)| {
                        (
                            fn_name.clone(),
                            FunctionVisibility {
                                public: is_public,
                                private: !is_public,
                            },
                        )
                    })
                    .collect();
                self.modules.insert(mod_name.clone(), vis_map);
            }
        }
    }

    pub(super) fn has_struct_module(&self, module_name: &str) -> bool {
        self.structs.contains_key(module_name)
    }

    pub(super) fn struct_has_field(&self, module_name: &str, field: &str) -> bool {
        self.structs
            .get(module_name)
            .is_some_and(|fields| fields.contains(field))
    }
}

fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err"
            | "tuple"
            | "list"
            | "map"
            | "keyword"
            | "protocol_dispatch"
            | "host_call"
            | "div"
            | "rem"
            | "byte_size"
            | "bit_size"
            | "abs"
            | "length"
            | "hd"
            | "tl"
            | "elem"
            | "tuple_size"
            | "to_string"
            | "max"
            | "min"
            | "round"
            | "trunc"
            | "map_size"
            | "put_elem"
            | "inspect"
    )
}
