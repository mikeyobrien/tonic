use crate::ir::{IrCallTarget, IrOp};
use crate::mir::{MirBinaryKind, MirBlock, MirFunction, MirInstruction};
use std::collections::BTreeMap;

use super::error::CBackendError;
use super::hash::{
    closure_capture_names, hash_closure_descriptor_i64, hash_ir_op_i64, hash_pattern_i64,
    hash_text_i64,
};

pub(super) fn emit_c_instructions(
    function: &MirFunction,
    block: &MirBlock,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    for instruction in &block.instructions {
        match instruction {
            MirInstruction::ConstInt { dest, value, .. } => {
                out.push_str(&format!("  v{dest} = (TnVal){value}LL;\n"));
            }
            MirInstruction::ConstBool { dest, value, .. } => {
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
            }
            MirInstruction::ConstNil { dest, .. } => {
                out.push_str(&format!("  v{dest} = tn_runtime_const_nil();\n"));
            }
            MirInstruction::ConstAtom { dest, value, .. } => {
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
            }
            MirInstruction::ConstString { dest, value, .. } => {
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
            }
            MirInstruction::ConstFloat { dest, value, .. } => {
                let escaped = c_string_literal(&value.to_string());
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
            }
            MirInstruction::LoadVariable { dest, name, .. } => {
                if let Some(param_index) = function.params.iter().position(|p| p.name == *name) {
                    out.push_str(&format!("  v{dest} = _arg{param_index};\n"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    out.push_str(&format!(
                        "  v{dest} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                }
            }
            MirInstruction::Unary {
                dest, kind, input, ..
            } => match kind {
                crate::mir::MirUnaryKind::Raise => {
                    out.push_str(&format!("  v{dest} = tn_runtime_raise(v{input});\n"));
                }
                crate::mir::MirUnaryKind::ToString => {
                    out.push_str(&format!("  v{dest} = tn_runtime_to_string(v{input});\n"));
                }
                crate::mir::MirUnaryKind::Not => {
                    out.push_str(&format!("  v{dest} = tn_runtime_not(v{input});\n"));
                }
                crate::mir::MirUnaryKind::Bang => {
                    out.push_str(&format!("  v{dest} = tn_runtime_bang(v{input});\n"));
                }
            },
            MirInstruction::Question { dest, input, .. } => {
                out.push_str(&format!("  v{dest} = tn_runtime_question(v{input});\n"));
            }
            MirInstruction::Binary {
                dest,
                kind,
                left,
                right,
                ..
            } => {
                emit_c_binary(*dest, kind, *left, *right, out);
            }
            MirInstruction::Call {
                dest,
                callee,
                args,
                offset,
                ..
            } => {
                let root_frame = format!("tn_root_frame_v{dest}");
                out.push_str(&format!(
                    "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                for arg in args {
                    out.push_str(&format!("  tn_runtime_root_register(v{arg});\n"));
                }

                emit_c_call(
                    *dest,
                    callee,
                    args,
                    callable_symbols,
                    &function.name,
                    *offset,
                    out,
                )?;
                out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
            }
            MirInstruction::CallValue {
                dest, callee, args, ..
            } => {
                let root_frame = format!("tn_root_frame_v{dest}");
                out.push_str(&format!(
                    "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register(v{callee});\n"));
                for arg in args {
                    out.push_str(&format!("  tn_runtime_root_register(v{arg});\n"));
                }

                // Variadic closure call via stub
                let all_args = std::iter::once(format!("v{callee}"))
                    .chain(std::iter::once(format!("(TnVal){}", args.len())))
                    .chain(args.iter().map(|a| format!("v{a}")))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_call_closure_varargs({all_args});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
            }
            MirInstruction::MakeClosure {
                dest, params, ops, ..
            } => {
                let capture_names = closure_capture_names(params, ops);
                let descriptor_hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_make_closure((TnVal){descriptor_hash}LL, (TnVal){}, (TnVal){});\n",
                    params.len(),
                    capture_names.len()
                ));
            }
            MirInstruction::MatchPattern {
                dest,
                input,
                pattern,
                ..
            } => {
                let pattern_hash = hash_pattern_i64(pattern)?;
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_match_operator(v{input}, (TnVal){pattern_hash}LL);\n"
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
                        return Err(CBackendError::unsupported_instruction(
                            &function.name,
                            instruction,
                            *offset,
                        ));
                    }
                };

                let op_hash = hash_ir_op_i64(source)?;
                let Some(dest) = dest else {
                    return Err(CBackendError::new(format!(
                        "c backend missing legacy destination in function {} at offset {offset}",
                        function.name
                    )));
                };
                out.push_str(&format!(
                    "  v{dest} = {runtime_helper}((TnVal){op_hash}LL);\n"
                ));
            }
        }
    }
    Ok(())
}

fn emit_c_binary(dest: u32, kind: &MirBinaryKind, left: u32, right: u32, out: &mut String) {
    match kind {
        MirBinaryKind::AddInt => out.push_str(&format!("  v{dest} = v{left} + v{right};\n")),
        MirBinaryKind::SubInt => out.push_str(&format!("  v{dest} = v{left} - v{right};\n")),
        MirBinaryKind::MulInt => out.push_str(&format!("  v{dest} = v{left} * v{right};\n")),
        MirBinaryKind::DivInt => out.push_str(&format!("  v{dest} = v{left} / v{right};\n")),
        MirBinaryKind::CmpIntEq => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} == v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::CmpIntNotEq => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} != v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::CmpIntLt => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} < v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::CmpIntLte => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} <= v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::CmpIntGt => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} > v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::CmpIntGte => out.push_str(&format!(
            "  v{dest} = tn_runtime_const_bool((v{left} >= v{right}) ? 1 : 0);\n"
        )),
        MirBinaryKind::Concat => out.push_str(&format!(
            "  v{dest} = tn_runtime_concat(v{left}, v{right});\n"
        )),
        MirBinaryKind::In => {
            out.push_str(&format!("  v{dest} = tn_runtime_in(v{left}, v{right});\n"))
        }
        MirBinaryKind::PlusPlus => out.push_str(&format!(
            "  v{dest} = tn_runtime_list_concat(v{left}, v{right});\n"
        )),
        MirBinaryKind::MinusMinus => out.push_str(&format!(
            "  v{dest} = tn_runtime_list_subtract(v{left}, v{right});\n"
        )),
        MirBinaryKind::Range => out.push_str(&format!(
            "  v{dest} = tn_runtime_range(v{left}, v{right});\n"
        )),
    }
}

fn emit_c_call(
    dest: u32,
    callee: &IrCallTarget,
    args: &[u32],
    callable_symbols: &BTreeMap<(String, usize), String>,
    function_name: &str,
    offset: usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("v{id}"))
        .collect::<Vec<_>>()
        .join(", ");

    match callee {
        IrCallTarget::Builtin { name } => {
            emit_c_builtin_call(dest, name, args, function_name, offset, out)?;
        }
        IrCallTarget::Function { name } => {
            let key = (name.clone(), args.len());
            if let Some(symbol) = callable_symbols.get(&key) {
                out.push_str(&format!("  v{dest} = {symbol}({rendered_args});\n"));
                return Ok(());
            }

            if callable_symbols
                .keys()
                .any(|(candidate, _)| candidate == name)
            {
                out.push_str(&format!("  v{dest} = tn_runtime_error_arity_mismatch();\n"));
                return Ok(());
            }

            return Err(CBackendError::new(format!(
                "c backend unknown function call target {name} in function {function_name} at offset {offset}"
            )));
        }
    }
    Ok(())
}

fn emit_c_builtin_call(
    dest: u32,
    builtin: &str,
    args: &[u32],
    function_name: &str,
    offset: usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("v{id}"))
        .collect::<Vec<_>>()
        .join(", ");

    match builtin {
        "ok" => {
            if args.len() != 1 {
                return Err(CBackendError::new(format!(
                    "c backend builtin ok arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_ok({rendered_args});\n"
            ));
        }
        "err" => {
            if args.len() != 1 {
                return Err(CBackendError::new(format!(
                    "c backend builtin err arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_err({rendered_args});\n"
            ));
        }
        "tuple" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_tuple({rendered_args});\n"
            ));
        }
        "list" => {
            // Variadic: first arg is count, then elements
            let count = args.len();
            let count_then_args = std::iter::once(format!("(TnVal){count}"))
                .chain(args.iter().map(|id| format!("v{id}")))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_list_varargs({count_then_args});\n"
            ));
        }
        "map_empty" => {
            out.push_str(&format!("  v{dest} = tn_runtime_map_empty();\n"));
        }
        "map" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_map({rendered_args});\n"
            ));
        }
        "map_put" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_put({rendered_args});\n"
            ));
        }
        "map_update" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_update({rendered_args});\n"
            ));
        }
        "map_access" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_access({rendered_args});\n"
            ));
        }
        "keyword" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_keyword({rendered_args});\n"
            ));
        }
        "keyword_append" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_keyword_append({rendered_args});\n"
            ));
        }
        "host_call" => {
            let count = args.len();
            let count_then_args = std::iter::once(format!("(TnVal){count}"))
                .chain(args.iter().map(|id| format!("v{id}")))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "  v{dest} = tn_runtime_host_call_varargs({count_then_args});\n"
            ));
        }
        "protocol_dispatch" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_protocol_dispatch({rendered_args});\n"
            ));
        }
        other => {
            return Err(CBackendError::new(format!(
                "c backend unsupported builtin call target {other} in function {function_name} at offset {offset}"
            )));
        }
    }
    Ok(())
}

fn c_string_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
