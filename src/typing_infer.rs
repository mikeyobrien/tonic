use super::diag::TypingError;
use super::{qualify_function_name, ConstraintSolver, FunctionSignature, Type};
use crate::guard_builtins;
use crate::parser::{BinaryOp, Expr, Pattern};
use std::collections::BTreeMap;

pub(super) fn infer_expression_type(
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
        Expr::Tuple { items, .. } | Expr::List { items, .. } | Expr::Bitstring { items, .. } => {
            for item in items {
                infer_expression_type(item, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                infer_expression_type(entry.key(), current_module, signatures, solver)?;
                infer_expression_type(entry.value(), current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Struct { entries, .. } => {
            for entry in entries {
                infer_expression_type(&entry.value, current_module, signatures, solver)?;
            }
            Ok(Type::Dynamic)
        }
        Expr::Keyword { entries, .. } => {
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
        Expr::StructUpdate { base, updates, .. } => {
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
        Expr::Call {
            callee,
            args,
            offset,
            ..
        } => infer_call_type(
            callee,
            args,
            Some(*offset),
            None,
            current_module,
            signatures,
            solver,
        ),
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
                    question_requires_result_hint(value),
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
                crate::parser::UnaryOp::Plus | crate::parser::UnaryOp::Minus => Ok(Type::Dynamic),
                crate::parser::UnaryOp::BitwiseNot => {
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
                BinaryOp::Plus
                | BinaryOp::Minus
                | BinaryOp::Mul
                | BinaryOp::IntDiv
                | BinaryOp::Rem => Ok(Type::Dynamic),
                BinaryOp::Div => Ok(Type::Dynamic),
                BinaryOp::Eq | BinaryOp::NotEq => Ok(Type::Bool),
                BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => Ok(Type::Bool),
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
                BinaryOp::NotIn => Ok(Type::Bool),
                BinaryOp::StrictEq | BinaryOp::StrictBangEq => Ok(Type::Bool),
                BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::BitwiseShiftLeft
                | BinaryOp::BitwiseShiftRight => {
                    solver.unify(Type::Int, left_type, Some(left.offset()))?;
                    solver.unify(Type::Int, right_type, Some(right.offset()))?;
                    Ok(Type::Int)
                }
                BinaryOp::PlusPlus | BinaryOp::MinusMinus => Ok(Type::Dynamic),
                BinaryOp::Range => {
                    solver.unify(Type::Int, left_type, Some(left.offset()))?;
                    solver.unify(Type::Int, right_type, Some(right.offset()))?;
                    Ok(Type::Dynamic)
                }
                BinaryOp::SteppedRange => Ok(Type::Dynamic),
            }
        }
        Expr::Pipe { left, right, .. } => {
            let piped_value_type = infer_expression_type(left, current_module, signatures, solver)?;

            if let Expr::Call {
                callee,
                args,
                offset,
                ..
            } = right.as_ref()
            {
                return infer_call_type(
                    callee,
                    args,
                    Some(*offset),
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
            reduce,
            body,
            ..
        } => {
            for generator in generators {
                infer_expression_type(generator.source(), current_module, signatures, solver)?;
                if let Some(guard) = generator.guard() {
                    let guard_type =
                        infer_expression_type(guard, current_module, signatures, solver)?;
                    solver.unify(Type::Bool, guard_type, Some(guard.offset()))?;
                }
            }
            if let Some(into_expr) = into {
                infer_expression_type(into_expr, current_module, signatures, solver)?;
            }
            if let Some(reduce_expr) = reduce {
                infer_expression_type(reduce_expr, current_module, signatures, solver)?;
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
        Expr::Block { exprs, .. } => {
            let mut last_type = Type::Nil;
            for sub_expr in exprs {
                last_type = infer_expression_type(sub_expr, current_module, signatures, solver)?;
            }
            Ok(last_type)
        }
    }
}

fn infer_call_type(
    callee: &str,
    args: &[Expr],
    call_offset: Option<usize>,
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

    if let Some(result_type) = infer_builtin_call_type(callee, &arg_types, call_offset, solver)? {
        return Ok(result_type);
    }

    let target_name = qualify_call_target(current_module, callee);
    let Some(signature) = signatures.get(&target_name) else {
        // Cross-module call not defined in this AST (e.g. from a prior REPL input).
        // The resolver already validated it exists; treat return type as Dynamic.
        if callee.contains('.') {
            return Ok(Type::Dynamic);
        }
        return Err(TypingError::new(format!(
            "unknown call target during inference: {target_name}"
        )));
    };

    let max_arity = signature.params.len();
    let min_arity = max_arity.saturating_sub(signature.default_count);

    if arg_types.len() < min_arity || arg_types.len() > max_arity {
        let accepted_arities = (min_arity..=max_arity).collect::<Vec<_>>();
        return Err(TypingError::arity_mismatch(
            &target_name,
            &accepted_arities,
            arg_types.len(),
            call_offset,
        ));
    }

    Ok(signature.return_type.clone())
}

fn question_requires_result_hint(value: &Expr) -> &'static str {
    match value {
        Expr::Call { .. }
        | Expr::FieldAccess { .. }
        | Expr::IndexAccess { .. }
        | Expr::Invoke { .. }
        | Expr::Pipe { .. }
        | Expr::Variable { .. } => {
            "make this expression return `ok(...)` or `err(...)`, or remove the trailing `?`"
        }
        _ => "wrap this value with `ok(...)` or `err(...)`, or remove the trailing `?`",
    }
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
        return Err(TypingError::host_call_key_type_mismatch(
            key_type.label(),
            key_offset,
        ));
    }

    Ok(())
}

fn infer_builtin_call_type(
    callee: &str,
    arg_types: &[Type],
    call_offset: Option<usize>,
    solver: &mut ConstraintSolver,
) -> Result<Option<Type>, TypingError> {
    if let Some(expected_arity) = guard_builtins::guard_builtin_arity(callee) {
        if arg_types.len() != expected_arity {
            return Err(TypingError::arity_mismatch(
                callee,
                &[expected_arity],
                arg_types.len(),
                call_offset,
            ));
        }

        return Ok(Some(Type::Bool));
    }

    match callee {
        "ok" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "ok",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }

            let ok_type = arg_types[0].clone();
            let err_type = solver.fresh_var();
            Ok(Some(Type::result(ok_type, err_type)))
        }
        "err" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "err",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }

            let ok_type = solver.fresh_var();
            let err_type = arg_types[0].clone();
            Ok(Some(Type::result(ok_type, err_type)))
        }
        "tuple" | "map" | "keyword" => {
            if arg_types.len() != 2 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[2],
                    arg_types.len(),
                    call_offset,
                ));
            }

            Ok(Some(Type::Dynamic))
        }
        "list" => Ok(Some(Type::Dynamic)),
        "protocol_dispatch" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "protocol_dispatch",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }

            Ok(Some(Type::Dynamic))
        }
        "host_call" => {
            // host_call requires at least 1 arg (the host key atom)
            // Returns dynamic since host functions can return any type
            if arg_types.is_empty() {
                return Err(TypingError::minimum_arity_mismatch(
                    "host_call",
                    1,
                    0,
                    "start with the host key atom, for example `host_call(:sum_ints, ...)`",
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "div" | "rem" => {
            if arg_types.len() != 2 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[2],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Int))
        }
        "byte_size" | "bit_size" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Int))
        }
        "abs" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "abs",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "length" | "tuple_size" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Int))
        }
        "hd" | "tl" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "elem" => {
            if arg_types.len() != 2 {
                return Err(TypingError::arity_mismatch(
                    "elem",
                    &[2],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "to_string" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "to_string",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::String))
        }
        "max" | "min" => {
            if arg_types.len() != 2 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[2],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "round" | "trunc" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    callee,
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Int))
        }
        "map_size" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "map_size",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Int))
        }
        "put_elem" => {
            if arg_types.len() != 3 {
                return Err(TypingError::arity_mismatch(
                    "put_elem",
                    &[3],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::Dynamic))
        }
        "inspect" => {
            if arg_types.len() != 1 {
                return Err(TypingError::arity_mismatch(
                    "inspect",
                    &[1],
                    arg_types.len(),
                    call_offset,
                ));
            }
            Ok(Some(Type::String))
        }
        _ => Ok(None),
    }
}

fn qualify_call_target(current_module: &str, callee: &str) -> String {
    if callee.contains('.') {
        callee.to_string()
    } else {
        qualify_function_name(current_module, callee)
    }
}
