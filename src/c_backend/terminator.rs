use crate::guard_builtins;
use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern};
use crate::mir::{MirBlock, MirFunction, MirTypedName};
use std::collections::BTreeMap;

use super::error::CBackendError;
use super::hash::hash_pattern_i64;

pub(super) fn emit_c_terminator_with_phi(
    function: &MirFunction,
    block: &MirBlock,
    phi_ids: &BTreeMap<u32, Vec<u32>>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    match &block.terminator {
        crate::mir::MirTerminator::Return { value, .. } => {
            out.push_str(&format!("  tn_runtime_retain(v{value});\n"));
            out.push_str("  tn_runtime_root_frame_pop(tn_function_root_frame);\n");
            out.push_str(&format!("  return v{value};\n"));
        }
        crate::mir::MirTerminator::Jump { target, args } => {
            // Assign to the phi registers of the target block before jumping.
            let empty = Vec::new();
            let target_phi_regs = phi_ids.get(target).unwrap_or(&empty);
            for (i, arg_val) in args.iter().enumerate() {
                if let Some(&phi_reg) = target_phi_regs.get(i) {
                    out.push_str(&format!("  v{phi_reg} = v{arg_val};\n"));
                }
            }
            out.push_str(&format!("  goto bb{target};\n"));
        }
        crate::mir::MirTerminator::ShortCircuit {
            op,
            condition,
            on_evaluate_rhs,
            on_short_circuit,
            ..
        } => {
            let cond_expr = format!("tn_runtime_is_truthy(v{condition})");
            let (true_target, false_target) = match op {
                crate::mir::MirShortCircuitOp::AndAnd | crate::mir::MirShortCircuitOp::And => {
                    (on_evaluate_rhs, on_short_circuit)
                }
                crate::mir::MirShortCircuitOp::OrOr | crate::mir::MirShortCircuitOp::Or => {
                    (on_short_circuit, on_evaluate_rhs)
                }
            };
            out.push_str(&format!(
                "  if ({cond_expr} != 0) {{ goto bb{true_target}; }} else {{ goto bb{false_target}; }}\n"
            ));
        }
        crate::mir::MirTerminator::Match {
            scrutinee,
            arms,
            offset,
        } => {
            emit_c_match_terminator(
                function,
                block,
                *scrutinee,
                arms,
                *offset,
                callable_symbols,
                out,
            )?;
        }
    }
    Ok(())
}

fn emit_c_match_terminator(
    function: &MirFunction,
    block: &MirBlock,
    scrutinee: u32,
    arms: &[crate::mir::MirMatchArm],
    _offset: usize,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    if arms.is_empty() {
        out.push_str("  tn_runtime_error_no_matching_clause();\n");
        out.push_str("  return 0; /* unreachable */\n");
        return Ok(());
    }

    for (arm_index, arm) in arms.iter().enumerate() {
        let cond = emit_c_pattern_condition(
            &function.name,
            &format!("v{scrutinee}"),
            &arm.pattern,
            &format!("match_b{}_arm{arm_index}", block.id),
            out,
        )?;

        let guard_cond = if let Some(guard_ops) = &arm.guard_ops {
            let guard_reg = format!("match_b{}_arm{arm_index}_guard", block.id);
            let guard_val = emit_c_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &guard_reg,
                callable_symbols,
                out,
            )?;
            Some(guard_val)
        } else {
            None
        };

        let full_cond = match guard_cond {
            Some(gc) => format!("({cond}) && ({gc})"),
            None => cond,
        };

        if arm_index + 1 == arms.len() {
            out.push_str(&format!(
                "  if ({full_cond}) {{ goto bb{}; }} else {{ tn_runtime_error_no_matching_clause(); return 0; }}\n",
                arm.target
            ));
        } else {
            out.push_str(&format!(
                "  if ({full_cond}) {{ goto bb{}; }}\n",
                arm.target
            ));
        }
    }
    Ok(())
}

/// Returns a C expression (as a string) that is non-zero when `pattern`
/// matches `scrutinee_expr` (a C expression string, e.g. `"v5"` or `"_arg0"`).
pub(super) fn emit_c_pattern_condition(
    _function_name: &str,
    scrutinee_expr: &str,
    pattern: &IrPattern,
    label: &str,
    out: &mut String,
) -> Result<String, CBackendError> {
    match pattern {
        IrPattern::Wildcard => Ok("1".to_string()),
        IrPattern::Integer { value } => Ok(format!("({scrutinee_expr} == {value}LL)")),
        IrPattern::Bool { value } => Ok(format!(
            "tn_runtime_value_equal({scrutinee_expr}, tn_runtime_const_bool((TnVal){}))",
            if *value { 1 } else { 0 }
        )),
        IrPattern::Nil => Ok(format!(
            "tn_runtime_value_equal({scrutinee_expr}, tn_runtime_const_nil())"
        )),
        _ => {
            let pattern_hash = hash_pattern_i64(pattern)?;
            let reg = format!("{label}_complex");
            out.push_str(&format!(
                "  int {reg} = tn_runtime_pattern_matches({scrutinee_expr}, (TnVal){pattern_hash}LL);\n"
            ));
            Ok(reg)
        }
    }
}

pub(super) fn emit_c_guard_condition(
    function_name: &str,
    guard_ops: &[IrOp],
    params: &[MirTypedName],
    label: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<String, CBackendError> {
    let mut stack: Vec<String> = Vec::new();

    for (index, op) in guard_ops.iter().enumerate() {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(param_index) = params.iter().position(|p| &p.name == name) {
                    stack.push(format!("_arg{param_index}"));
                } else {
                    let binding_hash = super::hash::hash_text_i64(name);
                    let reg = format!("{label}_load_{index}");
                    out.push_str(&format!(
                        "  TnVal {reg} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(reg);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let reg = format!("{label}_ci_{index}");
                out.push_str(&format!("  TnVal {reg} = (TnVal){value}LL;\n"));
                stack.push(reg);
            }
            IrOp::ConstBool { value, .. } => {
                let reg = format!("{label}_cb_{index}");
                out.push_str(&format!(
                    "  TnVal {reg} = (TnVal){};\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(reg);
            }
            IrOp::ConstNil { .. } => {
                let reg = format!("{label}_cn_{index}");
                out.push_str(&format!("  TnVal {reg} = 0;\n"));
                stack.push(reg);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let left = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let op_str = match kind {
                    CmpKind::Eq => "==",
                    CmpKind::NotEq => "!=",
                    CmpKind::Lt => "<",
                    CmpKind::Lte => "<=",
                    CmpKind::Gt => ">",
                    CmpKind::Gte => ">=",
                    CmpKind::StrictEq => "==",
                    CmpKind::StrictNotEq => "!=",
                };
                let reg = format!("{label}_cmp_{index}");
                out.push_str(&format!(
                    "  TnVal {reg} = ({left} {op_str} {right}) ? 1 : 0;\n"
                ));
                stack.push(reg);
            }
            IrOp::Bang { .. } => {
                // Convert any value to boolean: non-zero → 1, zero → 0
                let value = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let reg = format!("{label}_bang_{index}");
                out.push_str(&format!("  TnVal {reg} = ({value} != 0) ? 1 : 0;\n"));
                stack.push(reg);
            }
            IrOp::Not { .. } => {
                // Logical NOT: zero → 1, non-zero → 0
                let value = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let reg = format!("{label}_not_{index}");
                out.push_str(&format!("  TnVal {reg} = ({value} == 0) ? 1 : 0;\n"));
                stack.push(reg);
            }
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                if stack.len() < *argc {
                    return Err(CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    )));
                }
                let split_index = stack.len() - *argc;
                let call_args = stack.split_off(split_index);
                let rendered_args = call_args.join(", ");
                let reg = format!("{label}_call_{index}");

                match callee {
                    IrCallTarget::Function { name } => {
                        let target_key = (name.clone(), *argc);
                        if let Some(symbol) = callable_symbols.get(&target_key) {
                            out.push_str(&format!("  TnVal {reg} = {symbol}({rendered_args});\n"));
                        } else if callable_symbols
                            .keys()
                            .any(|(candidate, _)| candidate == name)
                        {
                            out.push_str(&format!(
                                "  TnVal {reg} = tn_runtime_error_arity_mismatch();\n"
                            ));
                        } else {
                            return Err(CBackendError::new(format!(
                                "c backend unknown guard call target {name} in function {function_name} at offset {offset}"
                            )));
                        }
                    }
                    IrCallTarget::Builtin { name } => {
                        if let Some(helper) = guard_builtins::c_helper_name(name) {
                            if call_args.len() != guard_builtins::GUARD_BUILTIN_ARITY {
                                return Err(CBackendError::new(format!(
                                    "c backend guard builtin {name} arity mismatch in function {function_name} at offset {offset}"
                                )));
                            }

                            out.push_str(&format!("  TnVal {reg} = {helper}({rendered_args});\n"));
                        } else {
                            out.push_str(&format!(
                                "  TnVal {reg} = tn_stub_abort(\"guard builtin {name}\");\n"
                            ));
                        }
                    }
                }
                stack.push(reg);
            }
            _ => {
                return Err(CBackendError::new(format!(
                    "c backend unsupported guard op in function {function_name}"
                )));
            }
        }
    }

    stack.pop().ok_or_else(|| {
        CBackendError::new(format!(
            "c backend empty guard stack in function {function_name}"
        ))
    })
}
