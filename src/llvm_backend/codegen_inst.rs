use super::*;

pub(super) fn emit_instructions(
    function: &MirFunction,
    block: &MirBlock,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    for instruction in &block.instructions {
        match instruction {
            MirInstruction::ConstInt { dest, value, .. } => {
                lines.push(format!("  {} = add i64 0, {value}", value_register(*dest)));
            }
            MirInstruction::ConstBool { dest, value, .. } => {
                lines.push(format!(
                    "  {} = add i64 0, {}",
                    value_register(*dest),
                    i64::from(*value)
                ));
            }
            MirInstruction::ConstNil { dest, .. } => {
                lines.push(format!("  {} = add i64 0, 0", value_register(*dest)));
            }
            MirInstruction::ConstAtom { dest, value, .. } => {
                let atom_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_atom(i64 {atom_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::ConstString { dest, value, .. } => {
                let string_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_string(i64 {string_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::ConstFloat { dest, value, .. } => {
                let float_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_float(i64 {float_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::LoadVariable { dest, name, .. } => {
                if let Some(param_index) =
                    function.params.iter().position(|param| param.name == *name)
                {
                    lines.push(format!(
                        "  {} = add i64 0, %arg{param_index}",
                        value_register(*dest)
                    ));
                } else {
                    let binding_hash = hash_text_i64(name);
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_load_binding(i64 {binding_hash})",
                        value_register(*dest)
                    ));
                }
            }
            MirInstruction::Unary {
                dest, kind, input, ..
            } => match kind {
                crate::mir::MirUnaryKind::Raise => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_raise(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::ToString => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_to_string(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::Not => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_not(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::Bang => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_bang(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::BitwiseNot => {
                    lines.push(format!(
                        "  {} = xor i64 {}, -1",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
            },
            MirInstruction::Question { dest, input, .. } => {
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_question(i64 {})",
                    value_register(*dest),
                    value_register(*input)
                ));
            }
            MirInstruction::Legacy {
                dest,
                source,
                offset,
                ..
            } => {
                let runtime_helper = match source {
                    IrOp::Try { .. } => "tn_runtime_try",
                    IrOp::For { .. } => "tn_runtime_for",
                    _ => {
                        return Err(LlvmBackendError::unsupported_instruction(
                            &function.name,
                            instruction,
                            *offset,
                        ));
                    }
                };

                let op_hash = hash_ir_op_i64(source)?;
                let Some(dest) = dest else {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend missing legacy destination in function {} at offset {}",
                        function.name, offset
                    )));
                };

                lines.push(format!(
                    "  {} = call i64 @{runtime_helper}(i64 {op_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::MakeClosure {
                dest, params, ops, ..
            } => {
                let capture_names = closure_capture_names(params, ops);
                let descriptor_hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_make_closure(i64 {descriptor_hash}, i64 {}, i64 {})",
                    value_register(*dest),
                    params.len(),
                    capture_names.len()
                ));
            }
            MirInstruction::CallValue {
                dest, callee, args, ..
            } => {
                let mut rendered_args = vec![
                    format!("i64 {}", value_register(*callee)),
                    format!("i64 {}", args.len()),
                ];
                rendered_args.extend(
                    args.iter()
                        .map(|arg| format!("i64 {}", value_register(*arg))),
                );
                lines.push(format!(
                    "  {} = call i64 (i64, i64, ...) @tn_runtime_call_closure({})",
                    value_register(*dest),
                    rendered_args.join(", ")
                ));
            }
            MirInstruction::Binary {
                dest,
                kind,
                left,
                right,
                offset: _,
                ..
            } => match kind {
                MirBinaryKind::AddInt
                | MirBinaryKind::SubInt
                | MirBinaryKind::MulInt
                | MirBinaryKind::DivInt
                | MirBinaryKind::IntDiv
                | MirBinaryKind::RemInt => {
                    let op = match kind {
                        MirBinaryKind::AddInt => "add",
                        MirBinaryKind::SubInt => "sub",
                        MirBinaryKind::MulInt => "mul",
                        MirBinaryKind::DivInt => "sdiv",
                        MirBinaryKind::IntDiv => "sdiv",
                        MirBinaryKind::RemInt => "srem",
                        _ => unreachable!(),
                    };

                    lines.push(format!(
                        "  {} = {op} i64 {}, {}",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
                MirBinaryKind::CmpIntEq
                | MirBinaryKind::CmpIntNotEq
                | MirBinaryKind::CmpIntLt
                | MirBinaryKind::CmpIntLte
                | MirBinaryKind::CmpIntGt
                | MirBinaryKind::CmpIntGte => {
                    let predicate = match kind {
                        MirBinaryKind::CmpIntEq => "eq",
                        MirBinaryKind::CmpIntNotEq => "ne",
                        MirBinaryKind::CmpIntLt => "slt",
                        MirBinaryKind::CmpIntLte => "sle",
                        MirBinaryKind::CmpIntGt => "sgt",
                        MirBinaryKind::CmpIntGte => "sge",
                        _ => unreachable!(),
                    };

                    lines.push(format!(
                        "  %cmp_{dest} = icmp {predicate} i64 {}, {}",
                        value_register(*left),
                        value_register(*right)
                    ));
                    lines.push(format!(
                        "  {} = zext i1 %cmp_{dest} to i64",
                        value_register(*dest)
                    ));
                }
                MirBinaryKind::Concat
                | MirBinaryKind::In
                | MirBinaryKind::PlusPlus
                | MirBinaryKind::MinusMinus
                | MirBinaryKind::Range
                | MirBinaryKind::NotIn
                | MirBinaryKind::SteppedRange => {
                    let helper = match kind {
                        MirBinaryKind::Concat => "tn_runtime_concat",
                        MirBinaryKind::In => "tn_runtime_in",
                        MirBinaryKind::PlusPlus => "tn_runtime_list_concat",
                        MirBinaryKind::MinusMinus => "tn_runtime_list_subtract",
                        MirBinaryKind::Range => "tn_runtime_range",
                        MirBinaryKind::NotIn => "tn_runtime_not_in",
                        MirBinaryKind::SteppedRange => "tn_runtime_stepped_range",
                        _ => unreachable!(),
                    };
                    lines.push(format!(
                        "  {} = call i64 @{helper}(i64 {}, i64 {})",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
                MirBinaryKind::BitwiseAnd
                | MirBinaryKind::BitwiseOr
                | MirBinaryKind::BitwiseXor
                | MirBinaryKind::BitwiseShiftLeft
                | MirBinaryKind::BitwiseShiftRight => {
                    let op = match kind {
                        MirBinaryKind::BitwiseAnd => "and",
                        MirBinaryKind::BitwiseOr => "or",
                        MirBinaryKind::BitwiseXor => "xor",
                        MirBinaryKind::BitwiseShiftLeft => "shl",
                        MirBinaryKind::BitwiseShiftRight => "ashr",
                        _ => unreachable!(),
                    };
                    lines.push(format!(
                        "  {} = {op} i64 {}, {}",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
            },
            MirInstruction::Call {
                dest,
                callee,
                args,
                offset,
                ..
            } => match callee {
                IrCallTarget::Builtin { name } => {
                    emit_builtin_call_from_value_ids(
                        *dest,
                        name,
                        args,
                        &function.name,
                        *offset,
                        lines,
                    )?;
                }
                IrCallTarget::Function { name } => {
                    let key = (name.clone(), args.len());
                    if let Some(symbol) = callable_symbols.get(&key) {
                        let rendered_args = args
                            .iter()
                            .map(|id| format!("i64 {}", value_register(*id)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        lines.push(format!(
                            "  {} = call i64 @{symbol}({rendered_args})",
                            value_register(*dest)
                        ));
                        continue;
                    }

                    if callable_symbols
                        .keys()
                        .any(|(candidate, _)| candidate == name)
                    {
                        lines.push(format!(
                            "  {} = call i64 @tn_runtime_error_arity_mismatch()",
                            value_register(*dest)
                        ));
                        continue;
                    }

                    return Err(LlvmBackendError::new(format!(
                        "llvm backend unknown function call target {name} in function {} at offset {offset}",
                        function.name
                    )));
                }
            },
            MirInstruction::MatchPattern {
                dest,
                input,
                pattern,
                ..
            } => {
                let pattern_hash = hash_pattern_i64(pattern)?;
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_match_operator(i64 {}, i64 {pattern_hash})",
                    value_register(*dest),
                    value_register(*input),
                ));
            }
        }
    }

    Ok(())
}
