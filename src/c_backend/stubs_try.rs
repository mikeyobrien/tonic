use std::collections::BTreeMap;

use crate::ir::{IrCallTarget, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::{hash_ir_op_i64, hash_pattern_i64, hash_text_i64};
use super::stubs::{c_string_literal, pop_stack_value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrySpec {
    pub(super) hash: i64,
    pub(super) op: IrOp,
}

pub(super) fn emit_runtime_try_helpers(
    mir: &MirProgram,
    out: &mut String,
) -> Result<(), CBackendError> {
    let try_specs = collect_try_specs(mir)?;

    out.push_str("/* compiled try helpers */\n");
    for (index, try_spec) in try_specs.iter().enumerate() {
        emit_runtime_try_case(index, try_spec, out)?;
    }

    out.push_str("static TnVal tn_runtime_try(TnVal op_hash) {\n");
    if try_specs.is_empty() {
        out.push_str("  return tn_stub_abort(\"tn_runtime_try\");\n");
    } else {
        out.push_str("  switch (op_hash) {\n");
        for (index, try_spec) in try_specs.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_runtime_try_case_{index}();\n",
                try_spec.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str("      return tn_stub_abort(\"tn_runtime_try\");\n");
        out.push_str("  }\n");
    }
    out.push_str("}\n\n");

    Ok(())
}

fn collect_try_specs(mir: &MirProgram) -> Result<Vec<TrySpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, IrOp>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::Legacy { source, .. } = instruction else {
                    continue;
                };

                if !matches!(source, IrOp::Try { .. }) {
                    continue;
                }

                let hash = hash_ir_op_i64(source)?;
                if let Some(existing) = by_hash.get(&hash) {
                    if existing != source {
                        return Err(CBackendError::new(format!(
                            "c backend try hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, source.clone());
                }
            }
        }
    }

    Ok(by_hash
        .into_iter()
        .map(|(hash, op)| TrySpec { hash, op })
        .collect())
}

fn emit_runtime_try_case(
    index: usize,
    try_spec: &TrySpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    let IrOp::Try {
        body_ops,
        rescue_branches,
        catch_branches,
        after_ops,
        ..
    } = &try_spec.op
    else {
        return Err(CBackendError::new(
            "c backend internal error: try case source was not IrOp::Try",
        ));
    };

    out.push_str(&format!(
        "static TnVal tn_runtime_try_case_{index}(void) {{\n"
    ));
    out.push_str("  TnBinding tn_try_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t tn_try_bindings_len = 0;\n");
    out.push_str("  tn_binding_snapshot(tn_try_bindings, &tn_try_bindings_len);\n");
    out.push_str("  int tn_try_raised = 0;\n");
    out.push_str("  TnVal tn_try_error = tn_runtime_const_nil();\n");
    out.push_str("  TnVal tn_try_result = tn_runtime_const_nil();\n");

    emit_try_ops(
        body_ops,
        "tn_try_result",
        "tn_try_raised",
        "tn_try_error",
        &format!("tn_try_case_{index}_body"),
        "  ",
        out,
    )?;

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    int tn_try_handled = 0;\n");

    for (branch_index, branch) in rescue_branches.iter().enumerate() {
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));

        // Determine indent for the branch body — deeper when a guard wraps it.
        let body_indent = if branch.guard_ops.is_some() {
            "        "
        } else {
            "      "
        };

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_label = format!("tn_try_case_{index}_rescue_{branch_index}_guard");
            out.push_str("      TnVal tn_guard_result = tn_runtime_const_nil();\n");
            out.push_str("      int tn_guard_raised = 0;\n");
            out.push_str("      TnVal tn_guard_error = tn_runtime_const_nil();\n");
            emit_try_ops(
                guard_ops,
                "tn_guard_result",
                "tn_guard_raised",
                "tn_guard_error",
                &guard_label,
                "      ",
                out,
            )?;
            out.push_str(
                "      if (tn_guard_raised == 0 && tn_runtime_is_truthy(tn_guard_result)) {\n",
            );
        }

        out.push_str(&format!("{body_indent}int tn_branch_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_error = tn_runtime_const_nil();\n"
        ));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_result = tn_runtime_const_nil();\n"
        ));
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_rescue_{branch_index}"),
            body_indent,
            out,
        )?;
        out.push_str(&format!("{body_indent}if (tn_branch_raised != 0) {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 1;\n"));
        out.push_str(&format!("{body_indent}  tn_try_error = tn_branch_error;\n"));
        out.push_str(&format!("{body_indent}}} else {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}  tn_try_result = tn_branch_result;\n"
        ));
        out.push_str(&format!("{body_indent}  tn_try_handled = 1;\n"));
        out.push_str(&format!("{body_indent}}}\n"));

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n");
    }

    for (branch_index, branch) in catch_branches.iter().enumerate() {
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_raised != 0 && tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));

        let body_indent = if branch.guard_ops.is_some() {
            "        "
        } else {
            "      "
        };

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_label = format!("tn_try_case_{index}_catch_{branch_index}_guard");
            out.push_str("      TnVal tn_guard_result = tn_runtime_const_nil();\n");
            out.push_str("      int tn_guard_raised = 0;\n");
            out.push_str("      TnVal tn_guard_error = tn_runtime_const_nil();\n");
            emit_try_ops(
                guard_ops,
                "tn_guard_result",
                "tn_guard_raised",
                "tn_guard_error",
                &guard_label,
                "      ",
                out,
            )?;
            out.push_str(
                "      if (tn_guard_raised == 0 && tn_runtime_is_truthy(tn_guard_result)) {\n",
            );
        }

        out.push_str(&format!("{body_indent}int tn_branch_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_error = tn_runtime_const_nil();\n"
        ));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_result = tn_runtime_const_nil();\n"
        ));
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_catch_{branch_index}"),
            body_indent,
            out,
        )?;
        out.push_str(&format!("{body_indent}if (tn_branch_raised != 0) {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 1;\n"));
        out.push_str(&format!("{body_indent}  tn_try_error = tn_branch_error;\n"));
        out.push_str(&format!("{body_indent}}} else {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}  tn_try_result = tn_branch_result;\n"
        ));
        out.push_str(&format!("{body_indent}  tn_try_handled = 1;\n"));
        out.push_str(&format!("{body_indent}}}\n"));

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n");
    }

    out.push_str("  }\n");

    if let Some(after_ops) = after_ops {
        out.push_str("  int tn_after_raised = 0;\n");
        out.push_str("  TnVal tn_after_error = tn_runtime_const_nil();\n");
        out.push_str("  TnVal tn_after_result = tn_runtime_const_nil();\n");
        emit_try_ops(
            after_ops,
            "tn_after_result",
            "tn_after_raised",
            "tn_after_error",
            &format!("tn_try_case_{index}_after"),
            "  ",
            out,
        )?;
        out.push_str("  if (tn_after_raised != 0) {\n");
        out.push_str("    tn_try_raised = 1;\n");
        out.push_str("    tn_try_error = tn_after_error;\n");
        out.push_str("  }\n");
    }

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    TnObj *err_obj = tn_get_obj(tn_try_error);\n");
    out.push_str("    tn_binding_restore(tn_try_bindings, tn_try_bindings_len);\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_STRING) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_ATOM) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    return tn_runtime_fail(\"exception raised\");\n");
    out.push_str("  }\n");

    out.push_str("  tn_binding_restore(tn_try_bindings, tn_try_bindings_len);\n");
    out.push_str("  return tn_try_result;\n");
    out.push_str("}\n\n");

    Ok(())
}

fn emit_try_ops(
    ops: &[IrOp],
    result_var: &str,
    raised_flag_var: &str,
    raised_value_var: &str,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!("{indent}do {{\n"));

    let mut stack = Vec::<String>::new();
    let mut temp_index = 0usize;
    let mut terminated = false;

    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("{indent}  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_nil();\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::LoadVariable { name, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let binding_hash = hash_text_i64(name);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                ));
                stack.push(temp);
            }
            IrOp::Raise { .. } => {
                let error_value = pop_stack_value(&mut stack, "try raise input")?;
                out.push_str(&format!("{indent}  {raised_flag_var} = 1;\n"));
                out.push_str(&format!("{indent}  {raised_value_var} = {error_value};\n"));
                out.push_str(&format!("{indent}  break;\n"));
                terminated = true;
                break;
            }
            IrOp::Return { .. } => {
                let return_value = pop_stack_value(&mut stack, "try return value")?;
                out.push_str(&format!("{indent}  {result_var} = {return_value};\n"));
                out.push_str(&format!("{indent}  break;\n"));
                terminated = true;
                break;
            }
            IrOp::Call { callee, argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(&mut stack, "try call argument")?);
                }
                args.reverse();
                let rendered_args = args.join(", ");
                let temp = format!("{label}_tmp_{temp_index}");
                let root_frame = format!("{label}_rf_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                for argument in &args {
                    out.push_str(&format!(
                        "{indent}  tn_runtime_root_register({argument});\n"
                    ));
                }
                match callee {
                    IrCallTarget::Builtin { name } => match name.as_str() {
                        "tuple" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_tuple({rendered_args});\n"
                            ));
                        }
                        "list" => {
                            let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                                .chain(args.iter().cloned())
                                .collect::<Vec<_>>()
                                .join(", ");
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_list_varargs({count_then_args});\n"
                            ));
                        }
                        "map_empty" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_map_empty();\n"
                            ));
                        }
                        "map" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_map({rendered_args});\n"
                            ));
                        }
                        "map_put" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_map_put({rendered_args});\n"
                            ));
                        }
                        "map_update" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_map_update({rendered_args});\n"
                            ));
                        }
                        "map_access" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_map_access({rendered_args});\n"
                            ));
                        }
                        "keyword" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_keyword({rendered_args});\n"
                            ));
                        }
                        "keyword_append" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_keyword_append({rendered_args});\n"
                            ));
                        }
                        "ok" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_ok({rendered_args});\n"
                            ));
                        }
                        "err" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_make_err({rendered_args});\n"
                            ));
                        }
                        "to_string" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_to_string({rendered_args});\n"
                            ));
                        }
                        "host_call" => {
                            let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                                .chain(args.iter().cloned())
                                .collect::<Vec<_>>()
                                .join(", ");
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_host_call_varargs({count_then_args});\n"
                            ));
                        }
                        "protocol_dispatch" => {
                            out.push_str(&format!(
                                "{indent}  TnVal {temp} = tn_runtime_protocol_dispatch({rendered_args});\n"
                            ));
                        }
                        other => {
                            return Err(CBackendError::new(format!(
                                "c backend try helper unsupported builtin call: {other}"
                            )));
                        }
                    },
                    IrCallTarget::Function { name } => {
                        let symbol = mangle_function_name(name, *argc);
                        out.push_str(&format!(
                            "{indent}  TnVal {temp} = {symbol}({rendered_args});\n"
                        ));
                    }
                }
                out.push_str(&format!("{indent}  tn_runtime_retain({temp});\n"));
                out.push_str(&format!(
                    "{indent}  tn_runtime_root_frame_pop({root_frame});\n"
                ));
                out.push_str(&format!("{indent}  tn_runtime_root_register({temp});\n"));
                out.push_str(&format!("{indent}  tn_runtime_release({temp});\n"));
                stack.push(temp);
            }
            IrOp::Case { branches, .. } => {
                let subject = pop_stack_value(&mut stack, "try case subject")?;
                let case_result = format!("{label}_case_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {case_result} = tn_runtime_const_nil();\n"
                ));

                let mut first = true;
                for (arm_index, arm) in branches.iter().enumerate() {
                    let arm_pattern_hash = hash_pattern_i64(&arm.pattern)?;
                    let condition = if matches!(arm.pattern, IrPattern::Wildcard) {
                        "1".to_string()
                    } else {
                        format!(
                            "tn_runtime_pattern_matches({subject}, (TnVal){arm_pattern_hash}LL)"
                        )
                    };
                    let keyword = if first { "if" } else { "} else if" };
                    first = false;
                    out.push_str(&format!("{indent}  {keyword} ({condition}) {{\n"));

                    let arm_label = format!("{label}_case_{}_arm_{arm_index}", temp_index - 1);
                    emit_try_ops(
                        &arm.ops,
                        &case_result,
                        raised_flag_var,
                        raised_value_var,
                        &arm_label,
                        &format!("{indent}    "),
                        out,
                    )?;
                }
                if !branches.is_empty() {
                    out.push_str(&format!("{indent}  }}\n"));
                }
                stack.push(case_result);
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend try helper unsupported op: {other:?}"
                )));
            }
        }
    }

    if !terminated {
        if let Some(value) = stack.pop() {
            out.push_str(&format!("{indent}  {result_var} = {value};\n"));
        } else {
            out.push_str(&format!(
                "{indent}  {result_var} = tn_runtime_const_nil();\n"
            ));
        }
    }

    out.push_str(&format!("{indent}}} while (0);\n"));
    Ok(())
}
