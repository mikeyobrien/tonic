use super::*;

pub(super) fn emit_terminator(
    function: &MirFunction,
    block: &MirBlock,
    blocks: &BTreeMap<u32, &MirBlock>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    match &block.terminator {
        MirTerminator::Return { value, .. } => {
            lines.push(format!("  ret i64 {}", value_register(*value)));
            Ok(())
        }
        MirTerminator::Jump { target, args } => {
            let Some(target_block) = blocks.get(target) else {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend missing jump target block {} in function {}",
                    target, function.name
                )));
            };

            if args.len() != target_block.args.len() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend jump argument mismatch into block {} in function {}",
                    target, function.name
                )));
            }

            lines.push(format!("  br label %bb{target}"));
            Ok(())
        }
        MirTerminator::ShortCircuit {
            op,
            condition,
            on_evaluate_rhs,
            on_short_circuit,
            ..
        } => {
            let condition_bool = format!("%sc_cond_{}", block.id);
            lines.push(format!(
                "  {condition_bool} = icmp ne i64 {}, 0",
                value_register(*condition)
            ));

            let (true_target, false_target) = match op {
                crate::mir::MirShortCircuitOp::AndAnd | crate::mir::MirShortCircuitOp::And => {
                    (on_evaluate_rhs, on_short_circuit)
                }
                crate::mir::MirShortCircuitOp::OrOr | crate::mir::MirShortCircuitOp::Or => {
                    (on_short_circuit, on_evaluate_rhs)
                }
            };

            lines.push(format!(
                "  br i1 {condition_bool}, label %bb{true_target}, label %bb{false_target}"
            ));
            Ok(())
        }
        MirTerminator::Match {
            scrutinee,
            arms,
            offset,
        } => emit_match_terminator(
            function,
            block,
            *scrutinee,
            arms,
            *offset,
            callable_symbols,
            lines,
        ),
    }
}

pub(super) fn emit_match_terminator(
    function: &MirFunction,
    block: &MirBlock,
    scrutinee: u32,
    arms: &[crate::mir::MirMatchArm],
    _offset: usize,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    if arms.is_empty() {
        lines.push(
            "  %match_no_clause = call i64 @tn_runtime_error_no_matching_clause()".to_string(),
        );
        lines.push("  ret i64 %match_no_clause".to_string());
        return Ok(());
    }

    let scrutinee_operand = value_register(scrutinee);

    for (arm_index, arm) in arms.iter().enumerate() {
        let pattern_condition = emit_pattern_condition(
            &function.name,
            &scrutinee_operand,
            &arm.pattern,
            &format!("match_block{}_arm{arm_index}_pattern", block.id),
            lines,
        )?;

        let mut condition_terms = vec![pattern_condition];
        if let Some(guard_ops) = &arm.guard_ops {
            let guard_condition = emit_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &format!("match_block{}_arm{arm_index}_guard", block.id),
                callable_symbols,
                lines,
            )?;
            condition_terms.push(guard_condition);
        }

        let condition = combine_conditions(
            &function.name,
            condition_terms,
            &format!("match_block{}_arm{arm_index}_condition", block.id),
            lines,
        )?;

        if arm_index + 1 == arms.len() {
            lines.push(format!(
                "  br i1 {condition}, label %bb{}, label %match_block{}_no_clause",
                arm.target, block.id
            ));
            lines.push(format!("match_block{}_no_clause:", block.id));
            lines.push(
                "  %match_no_clause = call i64 @tn_runtime_error_no_matching_clause()".to_string(),
            );
            lines.push("  ret i64 %match_no_clause".to_string());
        } else {
            lines.push(format!(
                "  br i1 {condition}, label %bb{}, label %match_block{}_arm{}_next",
                arm.target, block.id, arm_index
            ));
            lines.push(format!("match_block{}_arm{}_next:", block.id, arm_index));
        }
    }

    Ok(())
}

pub(super) fn hash_text_i64(value: &str) -> i64 {
    hash_bytes_i64(value.as_bytes())
}

pub(super) fn hash_pattern_i64(pattern: &IrPattern) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(pattern).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize pattern hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

pub(super) fn hash_ir_op_i64(op: &IrOp) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(op).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize ir op hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

pub(super) fn hash_closure_descriptor_i64(
    params: &[String],
    ops: &[IrOp],
    capture_names: &[String],
) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(&(params, ops, capture_names)).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize closure descriptor hash input: {error}"
        ))
    })?;

    Ok(hash_bytes_i64(serialized.as_bytes()))
}

pub(super) fn closure_capture_names(params: &[String], ops: &[IrOp]) -> Vec<String> {
    let mut captures = BTreeSet::new();
    let param_names = params.iter().cloned().collect::<BTreeSet<_>>();
    collect_capture_names_from_ops(ops, &param_names, &mut captures);
    captures.into_iter().collect()
}

pub(super) fn collect_capture_names_from_ops(
    ops: &[IrOp],
    params: &BTreeSet<String>,
    captures: &mut BTreeSet<String>,
) {
    for op in ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if !params.contains(name) {
                    captures.insert(name.clone());
                }
            }
            IrOp::AndAnd { right_ops, .. }
            | IrOp::OrOr { right_ops, .. }
            | IrOp::And { right_ops, .. }
            | IrOp::Or { right_ops, .. } => {
                collect_capture_names_from_ops(right_ops, params, captures);
            }
            IrOp::Case { branches, .. } => {
                for branch in branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
            }
            IrOp::Try {
                body_ops,
                rescue_branches,
                catch_branches,
                after_ops,
                ..
            } => {
                collect_capture_names_from_ops(body_ops, params, captures);
                for branch in rescue_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                for branch in catch_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                if let Some(after_ops) = after_ops {
                    collect_capture_names_from_ops(after_ops, params, captures);
                }
            }
            IrOp::For {
                generators,
                into_ops,
                reduce_ops,
                body_ops,
                ..
            } => {
                for generator in generators {
                    collect_capture_names_from_ops(&generator.source_ops, params, captures);
                    if let Some(guard_ops) = &generator.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                }
                if let Some(into_ops) = into_ops {
                    collect_capture_names_from_ops(into_ops, params, captures);
                }
                if let Some(reduce_ops) = reduce_ops {
                    collect_capture_names_from_ops(reduce_ops, params, captures);
                }
                collect_capture_names_from_ops(body_ops, params, captures);
            }
            IrOp::MakeClosure { .. }
            | IrOp::ConstInt { .. }
            | IrOp::ConstFloat { .. }
            | IrOp::ConstBool { .. }
            | IrOp::ConstNil { .. }
            | IrOp::ConstString { .. }
            | IrOp::ToString { .. }
            | IrOp::Call { .. }
            | IrOp::CallValue { .. }
            | IrOp::Question { .. }
            | IrOp::Raise { .. }
            | IrOp::ConstAtom { .. }
            | IrOp::AddInt { .. }
            | IrOp::SubInt { .. }
            | IrOp::MulInt { .. }
            | IrOp::DivInt { .. }
            | IrOp::IntDiv { .. }
            | IrOp::RemInt { .. }
            | IrOp::CmpInt { .. }
            | IrOp::Not { .. }
            | IrOp::Bang { .. }
            | IrOp::Concat { .. }
            | IrOp::In { .. }
            | IrOp::NotIn { .. }
            | IrOp::PlusPlus { .. }
            | IrOp::MinusMinus { .. }
            | IrOp::Range { .. }
            | IrOp::BitwiseAnd { .. }
            | IrOp::BitwiseOr { .. }
            | IrOp::BitwiseXor { .. }
            | IrOp::BitwiseNot { .. }
            | IrOp::BitwiseShiftLeft { .. }
            | IrOp::BitwiseShiftRight { .. }
            | IrOp::SteppedRange { .. }
            | IrOp::Bitstring { .. }
            | IrOp::Match { .. }
            | IrOp::Drop
            | IrOp::Return { .. } => {}
        }
    }
}

pub(super) fn hash_bytes_i64(bytes: &[u8]) -> i64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    i64::from_ne_bytes(hash.to_ne_bytes())
}
