use super::*;

pub(super) fn emit_builtin_call_from_value_ids(
    dest: u32,
    builtin: &str,
    args: &[u32],
    function_name: &str,
    offset: usize,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("i64 {}", value_register(*id)))
        .collect::<Vec<_>>();

    emit_builtin_call_from_registers(
        value_register(dest),
        builtin,
        rendered_args,
        function_name,
        offset,
        lines,
    )
}

pub(super) fn emit_builtin_call_from_registers(
    dest: String,
    builtin: &str,
    rendered_args: Vec<String>,
    function_name: &str,
    offset: usize,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    if let Some(helper) = guard_builtins::llvm_helper_name(builtin) {
        if rendered_args.len() != guard_builtins::GUARD_BUILTIN_ARITY {
            return Err(LlvmBackendError::new(format!(
                "llvm backend builtin {builtin} arity mismatch in function {function_name} at offset {offset}"
            )));
        }

        lines.push(format!(
            "  {dest} = call i64 @{helper}({})",
            rendered_args[0]
        ));
        return Ok(());
    }

    match builtin {
        "ok" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin ok arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_ok({})",
                rendered_args[0]
            ));
        }
        "err" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin err arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_err({})",
                rendered_args[0]
            ));
        }
        "tuple" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin tuple arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_tuple({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "list" => {
            let mut call_args = vec![format!("i64 {}", rendered_args.len())];
            call_args.extend(rendered_args);
            lines.push(format!(
                "  {dest} = call i64 (i64, ...) @tn_runtime_make_list({})",
                call_args.join(", ")
            ));
        }
        "bitstring" => {
            let mut call_args = vec![format!("i64 {}", rendered_args.len())];
            call_args.extend(rendered_args);
            lines.push(format!(
                "  {dest} = call i64 (i64, ...) @tn_runtime_make_bitstring({})",
                call_args.join(", ")
            ));
        }
        "map_empty" => {
            if !rendered_args.is_empty() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_empty arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!("  {dest} = call i64 @tn_runtime_map_empty()"));
        }
        "map" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_map({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "map_put" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_put arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_put({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "map_update" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_update arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_update({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "map_access" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_access arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_access({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "keyword" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin keyword arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_keyword({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "keyword_append" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin keyword_append arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_keyword_append({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "host_call" => {
            if rendered_args.is_empty() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin host_call arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            let mut call_args = vec![format!("i64 {}", rendered_args.len())];
            call_args.extend(rendered_args);
            lines.push(format!(
                "  {dest} = call i64 (i64, ...) @tn_runtime_host_call({})",
                call_args.join(", ")
            ));
        }
        "protocol_dispatch" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin protocol_dispatch arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_protocol_dispatch({})",
                rendered_args[0]
            ));
        }
        "div" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin div arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            // Integer division truncating toward zero (sdiv)
            lines.push(format!(
                "  {dest} = sdiv i64 {}, {}",
                rendered_args[0], rendered_args[1]
            ));
        }
        "rem" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin rem arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            // Integer remainder (srem)
            lines.push(format!(
                "  {dest} = srem i64 {}, {}",
                rendered_args[0], rendered_args[1]
            ));
        }
        "byte_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin byte_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_byte_size(i64 {})",
                rendered_args[0]
            ));
        }
        "bit_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin bit_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_bit_size(i64 {})",
                rendered_args[0]
            ));
        }
        "abs" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin abs arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_abs({})",
                rendered_args[0]
            ));
        }
        "length" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin length arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_length({})",
                rendered_args[0]
            ));
        }
        "hd" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin hd arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_hd({})",
                rendered_args[0]
            ));
        }
        "tl" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin tl arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_tl({})",
                rendered_args[0]
            ));
        }
        "elem" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin elem arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_elem({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "tuple_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin tuple_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_tuple_size({})",
                rendered_args[0]
            ));
        }
        "to_string" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin to_string arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_to_string({})",
                rendered_args[0]
            ));
        }
        "max" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin max arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_max({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "min" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin min arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_min({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "round" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin round arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_round({})",
                rendered_args[0]
            ));
        }
        "trunc" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin trunc arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_trunc({})",
                rendered_args[0]
            ));
        }
        "map_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_size({})",
                rendered_args[0]
            ));
        }
        "put_elem" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin put_elem arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_put_elem({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "inspect" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin inspect arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_inspect({})",
                rendered_args[0]
            ));
        }
        other => {
            return Err(LlvmBackendError::new(format!(
                "llvm backend unsupported builtin call target {other} in function {function_name} at offset {offset}"
            )));
        }
    }

    Ok(())
}
