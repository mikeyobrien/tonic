use crate::parser::{Ast, ModuleForm, ParameterAnnotation};
#[path = "typing_diag.rs"]
mod diag;
use diag::TypingError;
#[path = "typing_infer.rs"]
mod infer;
use infer::infer_expression_type;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSummary {
    signatures: BTreeMap<String, String>,
}

impl TypeSummary {
    /// Look up the inferred type signature for a fully-qualified function name
    /// (e.g. `"Demo.run"`).
    pub fn lookup(&self, name: &str) -> Option<&str> {
        self.signatures.get(name).map(String::as_str)
    }

    #[cfg(test)]
    pub fn signature(&self, name: &str) -> Option<&str> {
        self.signatures.get(name).map(String::as_str)
    }

    pub(crate) fn len(&self) -> usize {
        self.signatures.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    Int,
    Float,
    Bool,
    Nil,
    String,
    Dynamic,
    Result { ok: Box<Type>, err: Box<Type> },
    Var(TypeVarId),
}

impl Type {
    fn result(ok: Type, err: Type) -> Self {
        Self::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Float => "float",
            Type::Bool => "bool",
            Type::Nil => "nil",
            Type::String => "string",
            Type::Dynamic | Type::Var(_) => "dynamic",
            Type::Result { .. } => "result",
        }
    }

    fn label_for_question_requirement(&self) -> &'static str {
        match self {
            Type::Result { .. } => "Result",
            other => other.label(),
        }
    }
}

type TypeVarId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSignature {
    params: Vec<Type>,
    return_type: Type,
    default_count: usize,
}

#[derive(Debug, Default)]
struct ConstraintSolver {
    next_var: TypeVarId,
    substitutions: HashMap<TypeVarId, Type>,
}

impl ConstraintSolver {
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }

    fn unify(
        &mut self,
        expected: Type,
        found: Type,
        offset: Option<usize>,
    ) -> Result<(), TypingError> {
        let expected = self.resolve(expected);
        let found = self.resolve(found);

        match (expected, found) {
            (Type::Var(expected_id), Type::Var(found_id)) if expected_id == found_id => Ok(()),
            (Type::Var(id), ty) => {
                self.substitutions.insert(id, ty);
                Ok(())
            }
            (expected_ty, Type::Var(id)) => {
                self.substitutions.insert(id, expected_ty);
                Ok(())
            }
            (
                Type::Result {
                    ok: expected_ok,
                    err: expected_err,
                },
                Type::Result {
                    ok: found_ok,
                    err: found_err,
                },
            ) => {
                self.unify(*expected_ok, *found_ok, offset)?;
                self.unify(*expected_err, *found_err, offset)
            }
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::Nil, Type::Nil)
            | (Type::String, Type::String)
            | (Type::Dynamic, Type::Dynamic) => Ok(()),
            (expected_ty, found_ty) => Err(TypingError::type_mismatch(
                expected_ty.label(),
                found_ty.label(),
                offset,
            )),
        }
    }

    fn resolve(&mut self, ty: Type) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(mapped) = self.substitutions.get(&id).cloned() {
                    let resolved = self.resolve(mapped);
                    self.substitutions.insert(id, resolved.clone());
                    resolved
                } else {
                    Type::Var(id)
                }
            }
            Type::Result { ok, err } => Type::result(self.resolve(*ok), self.resolve(*err)),
            other => other,
        }
    }

    fn finalize(&mut self, ty: Type) -> Type {
        match self.resolve(ty) {
            Type::Var(_) => Type::Dynamic,
            Type::Result { ok, err } => Type::result(self.finalize(*ok), self.finalize(*err)),
            concrete => concrete,
        }
    }
}

pub fn infer_types(ast: &Ast) -> Result<TypeSummary, TypingError> {
    let mut solver = ConstraintSolver::default();
    let mut signatures: BTreeMap<String, FunctionSignature> = BTreeMap::new();

    for module in &ast.modules {
        for function in &module.functions {
            let params = function
                .params
                .iter()
                .map(|param| match param.annotation() {
                    ParameterAnnotation::Inferred => solver.fresh_var(),
                    ParameterAnnotation::Dynamic => Type::Dynamic,
                })
                .collect::<Vec<_>>();
            let default_count = function
                .params
                .iter()
                .rev()
                .take_while(|param| param.has_default())
                .count();
            let return_type = solver.fresh_var();
            let function_name = qualify_function_name(&module.name, &function.name);

            signatures
                .entry(function_name)
                .and_modify(|signature| {
                    signature.default_count = signature.default_count.max(default_count);
                })
                .or_insert(FunctionSignature {
                    params,
                    return_type,
                    default_count,
                });
        }

        for form in &module.forms {
            let ModuleForm::Defprotocol { name, functions } = form else {
                continue;
            };

            for function in functions {
                signatures
                    .entry(qualify_function_name(name, &function.name))
                    .or_insert(FunctionSignature {
                        params: function
                            .params
                            .iter()
                            .map(|_| solver.fresh_var())
                            .collect::<Vec<_>>(),
                        return_type: solver.fresh_var(),
                        default_count: 0,
                    });
            }
        }
    }

    for module in &ast.modules {
        for function in &module.functions {
            let function_name = qualify_function_name(&module.name, &function.name);
            let declared_return_type = signatures
                .get(&function_name)
                .expect("function signature should be pre-seeded")
                .return_type
                .clone();

            if let Some(signature) = signatures.get(&function_name) {
                for (index, parameter) in function.params.iter().enumerate() {
                    if let Some(default) = parameter.default() {
                        let default_type =
                            infer_expression_type(default, &module.name, &signatures, &mut solver)?;
                        solver.unify(
                            signature.params[index].clone(),
                            default_type,
                            Some(default.offset()),
                        )?;
                    }
                }
            }

            if let Some(guard) = function.guard() {
                let guard_type =
                    infer_expression_type(guard, &module.name, &signatures, &mut solver)?;
                solver.unify(Type::Bool, guard_type, Some(guard.offset()))?;
            }

            let inferred_body_type =
                infer_expression_type(&function.body, &module.name, &signatures, &mut solver)?;

            solver.unify(
                declared_return_type,
                inferred_body_type,
                Some(function.body.offset()),
            )?;
        }
    }

    let summary = signatures
        .into_iter()
        .map(|(name, signature)| {
            let params = signature
                .params
                .into_iter()
                .map(|param| solver.finalize(param))
                .collect::<Vec<_>>();
            let return_type = solver.finalize(signature.return_type);
            (name, format_signature(&params, &return_type))
        })
        .collect();

    Ok(TypeSummary {
        signatures: summary,
    })
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn format_signature(params: &[Type], return_type: &Type) -> String {
    let params = params
        .iter()
        .map(Type::label)
        .collect::<Vec<_>>()
        .join(", ");

    format!("fn({params}) -> {}", return_type.label())
}

#[cfg(test)]
mod tests;
