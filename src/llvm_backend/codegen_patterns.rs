use super::*;

pub(super) fn emit_pattern_condition(
    _function_name: &str,
    operand: &str,
    pattern: &IrPattern,
    label: &str,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    match pattern {
        IrPattern::Wildcard => Ok("true".to_string()),
        IrPattern::Integer { value } => {
            let register = format!("%{label}_int");
            lines.push(format!("  {register} = icmp eq i64 {operand}, {value}"));
            Ok(register)
        }
        IrPattern::Bool { value } => {
            let register = format!("%{label}_bool");
            lines.push(format!(
                "  {register} = icmp eq i64 {operand}, {}",
                i64::from(*value)
            ));
            Ok(register)
        }
        IrPattern::Nil => {
            let register = format!("%{label}_nil");
            lines.push(format!("  {register} = icmp eq i64 {operand}, 0"));
            Ok(register)
        }
        _ => {
            let pattern_hash = hash_pattern_i64(pattern)?;
            let register = format!("%{label}_complex");
            lines.push(format!(
                "  {register} = call i1 @tn_runtime_pattern_matches(i64 {operand}, i64 {pattern_hash})"
            ));
            Ok(register)
        }
    }
}

pub(super) fn emit_guard_condition(
    function_name: &str,
    guard_ops: &[IrOp],
    params: &[crate::mir::MirTypedName],
    label: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    let mut stack = Vec::<String>::new();

    for (index, op) in guard_ops.iter().enumerate() {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(param_index) = params.iter().position(|param| &param.name == name) {
                    stack.push(format!("%arg{param_index}"));
                } else {
                    let register = format!("%{label}_load_binding_{index}");
                    let binding_hash = hash_text_i64(name);
                    lines.push(format!(
                        "  {register} = call i64 @tn_runtime_load_binding(i64 {binding_hash})"
                    ));
                    stack.push(register);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let register = format!("%{label}_const_int_{index}");
                lines.push(format!("  {register} = add i64 0, {value}"));
                stack.push(register);
            }
            IrOp::ConstBool { value, .. } => {
                let register = format!("%{label}_const_bool_{index}");
                lines.push(format!("  {register} = add i64 0, {}", i64::from(*value)));
                stack.push(register);
            }
            IrOp::ConstNil { .. } => {
                let register = format!("%{label}_const_nil_{index}");
                lines.push(format!("  {register} = add i64 0, 0"));
                stack.push(register);
            }
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                if stack.len() < *argc {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    )));
                }

                let split_index = stack.len() - *argc;
                let call_args = stack.split_off(split_index);
                let rendered_args = call_args
                    .iter()
                    .map(|arg| format!("i64 {arg}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let result_register = format!("%{label}_call_{index}");
                match callee {
                    IrCallTarget::Function { name } => {
                        let target_key = (name.clone(), *argc);
                        if let Some(symbol) = callable_symbols.get(&target_key) {
                            lines.push(format!(
                                "  {result_register} = call i64 @{symbol}({rendered_args})"
                            ));
                        } else if callable_symbols
                            .keys()
                            .any(|(candidate, _)| candidate == name)
                        {
                            lines.push(format!(
                                "  {result_register} = call i64 @tn_runtime_error_arity_mismatch()"
                            ));
                        } else {
                            return Err(LlvmBackendError::new(format!(
                                "llvm backend unknown guard call target {name} in function {function_name} at offset {offset}"
                            )));
                        }
                    }
                    IrCallTarget::Builtin { name } => {
                        emit_builtin_call_from_registers(
                            result_register.clone(),
                            name,
                            call_args,
                            function_name,
                            *offset,
                            lines,
                        )?;
                    }
                }

                stack.push(result_register);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let left = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;

                let predicate = match kind {
                    CmpKind::Eq | CmpKind::StrictEq => "eq",
                    CmpKind::NotEq | CmpKind::StrictNotEq => "ne",
                    CmpKind::Lt => "slt",
                    CmpKind::Lte => "sle",
                    CmpKind::Gt => "sgt",
                    CmpKind::Gte => "sge",
                };

                let cmp_register = format!("%{label}_cmp_{index}");
                let cmp_value = format!("%{label}_cmp_value_{index}");
                lines.push(format!(
                    "  {cmp_register} = icmp {predicate} i64 {left}, {right}"
                ));
                lines.push(format!("  {cmp_value} = zext i1 {cmp_register} to i64"));
                stack.push(cmp_value);
            }
            IrOp::Bang { .. } => {
                let value = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let truthy = format!("%{label}_bang_truthy_{index}");
                let bang_value = format!("%{label}_bang_value_{index}");
                lines.push(format!("  {truthy} = icmp ne i64 {value}, 0"));
                lines.push(format!("  {bang_value} = zext i1 {truthy} to i64"));
                stack.push(bang_value);
            }
            IrOp::Not { .. } => {
                let value = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let strict = format!("%{label}_not_strict_{index}");
                let not_value = format!("%{label}_not_value_{index}");
                lines.push(format!("  {strict} = icmp eq i64 {value}, 0"));
                lines.push(format!("  {not_value} = zext i1 {strict} to i64"));
                stack.push(not_value);
            }
            other => {
                return Err(LlvmBackendError::unsupported_guard_op(
                    function_name,
                    other,
                    0,
                ));
            }
        }
    }

    let Some(final_value) = stack.pop() else {
        return Err(LlvmBackendError::new(format!(
            "llvm backend guard stack underflow in function {function_name}"
        )));
    };

    if !stack.is_empty() {
        return Err(LlvmBackendError::new(format!(
            "llvm backend guard stack leftover values in function {function_name}"
        )));
    }

    let condition = format!("%{label}_truthy");
    lines.push(format!("  {condition} = icmp ne i64 {final_value}, 0"));
    Ok(condition)
}

pub(super) fn combine_conditions(
    function_name: &str,
    conditions: Vec<String>,
    label: &str,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    if conditions.is_empty() {
        return Ok("true".to_string());
    }

    let mut iter = conditions.into_iter();
    let Some(mut current) = iter.next() else {
        return Err(LlvmBackendError::new(format!(
            "llvm backend missing condition in function {function_name}"
        )));
    };

    for (index, condition) in iter.enumerate() {
        let combined = format!("%{label}_and_{index}");
        lines.push(format!("  {combined} = and i1 {current}, {condition}"));
        current = combined;
    }

    Ok(current)
}

pub(super) fn instruction_destination(instruction: &MirInstruction) -> Option<u32> {
    match instruction {
        MirInstruction::ConstInt { dest, .. }
        | MirInstruction::ConstFloat { dest, .. }
        | MirInstruction::ConstBool { dest, .. }
        | MirInstruction::ConstNil { dest, .. }
        | MirInstruction::ConstString { dest, .. }
        | MirInstruction::ConstAtom { dest, .. }
        | MirInstruction::LoadVariable { dest, .. }
        | MirInstruction::Unary { dest, .. }
        | MirInstruction::Binary { dest, .. }
        | MirInstruction::Call { dest, .. }
        | MirInstruction::CallValue { dest, .. }
        | MirInstruction::MakeClosure { dest, .. }
        | MirInstruction::Question { dest, .. }
        | MirInstruction::MatchPattern { dest, .. } => Some(*dest),
        MirInstruction::Legacy { dest, .. } => *dest,
    }
}

pub(super) fn instruction_operands(instruction: &MirInstruction) -> Vec<u32> {
    match instruction {
        MirInstruction::Unary { input, .. } => vec![*input],
        MirInstruction::Binary { left, right, .. } => vec![*left, *right],
        MirInstruction::Call { args, .. } => args.clone(),
        MirInstruction::CallValue { callee, args, .. } => {
            let mut values = vec![*callee];
            values.extend(args.iter().copied());
            values
        }
        MirInstruction::Question { input, .. } => vec![*input],
        MirInstruction::MatchPattern { input, .. } => vec![*input],
        _ => Vec::new(),
    }
}

pub(super) fn terminator_operands(terminator: &MirTerminator) -> Vec<u32> {
    match terminator {
        MirTerminator::Return { value, .. } => vec![*value],
        MirTerminator::Jump { args, .. } => args.clone(),
        MirTerminator::Match { scrutinee, .. } => vec![*scrutinee],
        MirTerminator::ShortCircuit { condition, .. } => vec![*condition],
    }
}
