use crate::parser::{Ast, BinaryOp, Expr, ParameterAnnotation, Pattern};
#[path = "typing_diag.rs"]
mod diag;
use diag::TypingError;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSummary {
    signatures: BTreeMap<String, String>,
}

impl TypeSummary {
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

fn infer_expression_type(
    expr: &Expr,
    current_module: &str,
    signatures: &BTreeMap<String, FunctionSignature>,
    solver: &mut ConstraintSolver,
) -> Result<Type, TypingError> {
    match expr {
        Expr::Int { .. } => Ok(Type::Int),
        Expr::Float { .. } => Ok(Type::Float),
        Expr::Bool { .. } => Ok(Type::Bool),
        Expr::Nil { .. } => Ok(Type::Nil),
        Expr::String { .. } => Ok(Type::String),
        Expr::InterpolatedString { segments, .. } => {
            for segment in segments {
                if let crate::parser::InterpolationSegment::Expr { expr } = segment {
                    infer_expression_type(expr, current_module, signatures, solver)?;
                }
            }
            Ok(Type::String)
        }
        Expr::Tuple { items, .. } | Expr::List { items, .. } => {
            for item in items {
                infer_expression_type(item, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Map { entries, .. } | Expr::Keyword { entries, .. } => {
            for entry in entries {
                infer_expression_type(&entry.value, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::MapUpdate { base, updates, .. } => {
            infer_expression_type(base, current_module, signatures, solver)?;
            for entry in updates {
                infer_expression_type(&entry.value, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::FieldAccess { base, .. } => {
            infer_expression_type(base, current_module, signatures, solver)?;
            Ok(solver.fresh_var())
        }
        Expr::IndexAccess { base, index, .. } => {
            infer_expression_type(base, current_module, signatures, solver)?;
            infer_expression_type(index, current_module, signatures, solver)?;
            Ok(solver.fresh_var())
        }
        Expr::Call { callee, args, .. } => {
            infer_call_type(callee, args, None, current_module, signatures, solver)
        }
        Expr::Fn { body, .. } => {
            infer_expression_type(body, current_module, signatures, solver)?;
            Ok(Type::Dynamic)
        }
        Expr::Invoke { callee, args, .. } => {
            infer_expression_type(callee, current_module, signatures, solver)?;
            for arg in args {
                infer_expression_type(arg, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Question { value, offset, .. } => {
            let value_type = infer_expression_type(value, current_module, signatures, solver)?;
            let resolved_value_type = solver.resolve(value_type);

            match resolved_value_type {
                Type::Result { ok, .. } => Ok(*ok),
                Type::Var(var_id) => {
                    let ok_type = solver.fresh_var();
                    let err_type = solver.fresh_var();
                    solver.unify(
                        Type::Var(var_id),
                        Type::result(ok_type.clone(), err_type),
                        Some(*offset),
                    )?;
                    Ok(ok_type)
                }
                other => Err(TypingError::question_requires_result(
                    other.label_for_question_requirement(),
                    Some(*offset),
                )),
            }
        }
        Expr::Unary { op, value, .. } => {
            let value_type = infer_expression_type(value, current_module, signatures, solver)?;
            match op {
                crate::parser::UnaryOp::Not => {
                    solver.unify(Type::Bool, value_type, Some(value.offset()))?;
                    Ok(Type::Bool)
                }
                crate::parser::UnaryOp::Bang => Ok(Type::Bool),
                crate::parser::UnaryOp::Plus | crate::parser::UnaryOp::Minus => {
                    solver.unify(Type::Int, value_type, Some(value.offset()))?;
                    Ok(Type::Int)
                }
            }
        }
        Expr::Binary {
            op, left, right, ..
        } => {
            let left_type = infer_expression_type(left, current_module, signatures, solver)?;
            let right_type = infer_expression_type(right, current_module, signatures, solver)?;

            match op {
                BinaryOp::Match => Ok(right_type),
                BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Mul | BinaryOp::Div => {
                    solver.unify(Type::Int, left_type, Some(left.offset()))?;
                    solver.unify(Type::Int, right_type, Some(right.offset()))?;
                    Ok(Type::Int)
                }
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::Lte
                | BinaryOp::Gt
                | BinaryOp::Gte => {
                    solver.unify(Type::Int, left_type, Some(left.offset()))?;
                    solver.unify(Type::Int, right_type, Some(right.offset()))?;
                    Ok(Type::Bool)
                }
                BinaryOp::AndAnd | BinaryOp::OrOr => Ok(Type::Dynamic),
                BinaryOp::And | BinaryOp::Or => {
                    solver.unify(Type::Bool, left_type, Some(left.offset()))?;
                    Ok(Type::Dynamic)
                }
                BinaryOp::Concat => {
                    solver.unify(Type::String, left_type, Some(left.offset()))?;
                    solver.unify(Type::String, right_type, Some(right.offset()))?;
                    Ok(Type::String)
                }
                BinaryOp::In => Ok(Type::Bool),
                BinaryOp::PlusPlus | BinaryOp::MinusMinus => Ok(Type::Dynamic),
                BinaryOp::Range => {
                    solver.unify(Type::Int, left_type, Some(left.offset()))?;
                    solver.unify(Type::Int, right_type, Some(right.offset()))?;
                    Ok(Type::Dynamic)
                }
            }
        }
        Expr::Pipe { left, right, .. } => {
            let piped_value_type = infer_expression_type(left, current_module, signatures, solver)?;

            if let Expr::Call { callee, args, .. } = right.as_ref() {
                return infer_call_type(
                    callee,
                    args,
                    Some(piped_value_type),
                    current_module,
                    signatures,
                    solver,
                );
            }

            infer_expression_type(right, current_module, signatures, solver)
        }
        Expr::Case {
            subject,
            branches,
            offset,
            ..
        } => {
            infer_expression_type(subject, current_module, signatures, solver)?;

            if !branches
                .iter()
                .any(|branch| matches!(branch.head(), Pattern::Wildcard | Pattern::Bind { .. }))
            {
                return Err(TypingError::non_exhaustive_case(Some(*offset)));
            }

            let mut inferred_case_type = None;

            for branch in branches {
                if let Some(guard) = branch.guard() {
                    let guard_type =
                        infer_expression_type(guard, current_module, signatures, solver)?;
                    solver.unify(Type::Bool, guard_type, Some(guard.offset()))?;
                }

                let branch_type =
                    infer_expression_type(branch.body(), current_module, signatures, solver)?;

                if let Some(existing) = inferred_case_type.clone() {
                    solver.unify(existing, branch_type, Some(branch.body().offset()))?;
                } else {
                    inferred_case_type = Some(branch_type);
                }
            }

            Ok(inferred_case_type.unwrap_or(Type::Dynamic))
        }
        Expr::For {
            generators,
            into,
            body,
            ..
        } => {
            for (_, generator) in generators {
                infer_expression_type(generator, current_module, signatures, solver)?;
            }
            if let Some(into_expr) = into {
                infer_expression_type(into_expr, current_module, signatures, solver)?;
            }
            infer_expression_type(body, current_module, signatures, solver)?;
            Ok(Type::Dynamic)
        }
        Expr::Group { inner, .. } => {
            infer_expression_type(inner, current_module, signatures, solver)
        }
        Expr::Variable { .. } => Ok(solver.fresh_var()),
        Expr::Atom { .. } => Ok(Type::Dynamic),
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            infer_expression_type(body, current_module, signatures, solver)?;
            for branch in rescue {
                infer_expression_type(branch.body(), current_module, signatures, solver)?;
            }
            for branch in catch {
                infer_expression_type(branch.body(), current_module, signatures, solver)?;
            }
            if let Some(after) = after {
                infer_expression_type(after, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Raise { error, .. } => {
            infer_expression_type(error, current_module, signatures, solver)?;
            Ok(Type::Dynamic)
        }
    }
}

fn infer_call_type(
    callee: &str,
    args: &[Expr],
    piped_value_type: Option<Type>,
    current_module: &str,
    signatures: &BTreeMap<String, FunctionSignature>,
    solver: &mut ConstraintSolver,
) -> Result<Type, TypingError> {
    let has_piped_value = piped_value_type.is_some();
    let mut arg_types = Vec::with_capacity(args.len() + usize::from(has_piped_value));

    if let Some(piped_value_type) = piped_value_type {
        arg_types.push(piped_value_type);
    }

    for arg in args {
        arg_types.push(infer_expression_type(
            arg,
            current_module,
            signatures,
            solver,
        )?);
    }

    validate_host_call_key_type(callee, &arg_types, args, has_piped_value, solver)?;

    if let Some(result_type) = infer_builtin_call_type(callee, &arg_types, solver)? {
        return Ok(result_type);
    }

    let target_name = qualify_call_target(current_module, callee);
    let signature = signatures.get(&target_name).ok_or_else(|| {
        TypingError::new(format!(
            "unknown call target during inference: {target_name}"
        ))
    })?;

    let max_arity = signature.params.len();
    let min_arity = max_arity.saturating_sub(signature.default_count);

    if arg_types.len() < min_arity || arg_types.len() > max_arity {
        let expected = if min_arity == max_arity {
            max_arity.to_string()
        } else {
            format!("{min_arity}..{max_arity}")
        };

        return Err(TypingError::new(format!(
            "arity mismatch for {target_name}: expected {expected} args, found {}",
            arg_types.len()
        )));
    }

    Ok(signature.return_type.clone())
}

fn validate_host_call_key_type(
    callee: &str,
    arg_types: &[Type],
    args: &[Expr],
    has_piped_value: bool,
    solver: &mut ConstraintSolver,
) -> Result<(), TypingError> {
    if callee != "host_call" || arg_types.is_empty() {
        return Ok(());
    }

    let key_type = solver.resolve(arg_types[0].clone());
    let key_offset = if has_piped_value {
        None
    } else {
        args.first().map(Expr::offset)
    };

    if matches!(
        key_type,
        Type::Int | Type::Float | Type::Bool | Type::Nil | Type::String | Type::Result { .. }
    ) {
        return Err(TypingError::type_mismatch(
            "atom",
            key_type.label(),
            key_offset,
        ));
    }

    Ok(())
}

fn infer_builtin_call_type(
    callee: &str,
    arg_types: &[Type],
    solver: &mut ConstraintSolver,
) -> Result<Option<Type>, TypingError> {
    match callee {
        "ok" => {
            if arg_types.len() != 1 {
                return Err(TypingError::new(format!(
                    "arity mismatch for ok: expected 1 args, found {}",
                    arg_types.len()
                )));
            }

            let ok_type = arg_types[0].clone();
            let err_type = solver.fresh_var();
            Ok(Some(Type::result(ok_type, err_type)))
        }
        "err" => {
            if arg_types.len() != 1 {
                return Err(TypingError::new(format!(
                    "arity mismatch for err: expected 1 args, found {}",
                    arg_types.len()
                )));
            }

            let ok_type = solver.fresh_var();
            let err_type = arg_types[0].clone();
            Ok(Some(Type::result(ok_type, err_type)))
        }
        "tuple" | "map" | "keyword" => {
            if arg_types.len() != 2 {
                return Err(TypingError::new(format!(
                    "arity mismatch for {callee}: expected 2 args, found {}",
                    arg_types.len()
                )));
            }

            Ok(Some(Type::Dynamic))
        }
        "list" => Ok(Some(Type::Dynamic)),
        "protocol_dispatch" => {
            if arg_types.len() != 1 {
                return Err(TypingError::new(format!(
                    "arity mismatch for protocol_dispatch: expected 1 args, found {}",
                    arg_types.len()
                )));
            }

            Ok(Some(Type::Dynamic))
        }
        "host_call" => {
            // host_call requires at least 1 arg (the host key atom)
            // Returns dynamic since host functions can return any type
            if arg_types.is_empty() {
                return Err(TypingError::new(
                    "host_call requires at least 1 argument (host function key)",
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        _ => Ok(None),
    }
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn qualify_call_target(current_module: &str, callee: &str) -> String {
    if callee.contains('.') {
        callee.to_string()
    } else {
        qualify_function_name(current_module, callee)
    }
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
