use crate::ir::{IrCallTarget, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;

use super::super::error::CBackendError;
use super::super::hash::{hash_pattern_i64, hash_text_i64};
use super::super::stubs::{c_string_literal, pop_stack_value};

pub(super) fn emit_try_ops(
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
