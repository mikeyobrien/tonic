use std::collections::BTreeMap;

use crate::ir::{CmpKind, IrCallTarget, IrOp};
use crate::llvm_backend::mangle_function_name;
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::{closure_capture_names, hash_closure_descriptor_i64, hash_text_i64};
use super::stubs::{c_string_literal, pop_stack_value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClosureSpec {
    pub(super) hash: i64,
    pub(super) params: Vec<String>,
    pub(super) ops: Vec<IrOp>,
}
pub(super) fn emit_compiled_closure_helpers(
    mir: &MirProgram,
    out: &mut String,
) -> Result<(), CBackendError> {
    let closures = collect_closure_specs(mir)?;

    out.push_str("/* compiled closure helpers */\n");

    for (index, closure) in closures.iter().enumerate() {
        emit_compiled_closure_body(index, closure, out)?;
    }

    out.push_str(
        "static TnVal tn_runtime_call_compiled_closure(TnVal descriptor_hash, const TnVal *argv, size_t argc) {\n",
    );
    if closures.is_empty() {
        out.push_str(
            "  return tn_runtime_failf(\"unsupported closure descriptor %lld\", (long long)descriptor_hash);\n",
        );
    } else {
        out.push_str("  switch (descriptor_hash) {\n");
        for (index, closure) in closures.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_compiled_closure_{index}(argv, argc);\n",
                closure.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str(
            "      return tn_runtime_failf(\"unsupported closure descriptor %lld\", (long long)descriptor_hash);\n",
        );
        out.push_str("  }\n");
    }
    out.push_str("}\n");

    Ok(())
}

fn collect_closure_specs(mir: &MirProgram) -> Result<Vec<ClosureSpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, ClosureSpec>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::MakeClosure { params, ops, .. } = instruction else {
                    continue;
                };

                let capture_names = closure_capture_names(params, ops);
                let hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;

                let spec = ClosureSpec {
                    hash,
                    params: params.clone(),
                    ops: ops.clone(),
                };

                if let Some(existing) = by_hash.get(&hash) {
                    if existing != &spec {
                        return Err(CBackendError::new(format!(
                            "c backend closure descriptor hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, spec);
                }
            }
        }
    }

    Ok(by_hash.into_values().collect())
}

fn emit_compiled_closure_body(
    index: usize,
    closure: &ClosureSpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "static TnVal tn_compiled_closure_{index}(const TnVal *argv, size_t argc) {{\n"
    ));
    out.push_str(&format!("  if (argc != {}) {{\n", closure.params.len()));
    out.push_str(&format!(
        "    return tn_runtime_failf(\"arity mismatch for anonymous function: expected %d args, found %zu\", {}, argc);\n",
        closure.params.len()
    ));
    out.push_str("  }\n\n");
    out.push_str("  size_t tn_closure_root_frame = tn_runtime_root_frame_push();\n");
    out.push_str("  TnBinding tn_closure_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t tn_closure_bindings_len = 0;\n");
    out.push_str("  tn_binding_snapshot(tn_closure_bindings, &tn_closure_bindings_len);\n\n");

    let mut params = BTreeMap::<String, usize>::new();
    for (position, name) in closure.params.iter().enumerate() {
        params.insert(name.clone(), position);
    }

    let mut stack = Vec::<String>::new();
    let mut temp_index = 0usize;
    let mut emitted_return = false;

    for op in &closure.ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(position) = params.get(name) {
                    stack.push(format!("argv[{position}]"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    let temp = format!("tmp_{temp_index}");
                    temp_index += 1;
                    out.push_str(&format!(
                        "  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(temp);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_const_nil();\n"));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::AddInt { .. } => {
                emit_closure_binary("+", &mut stack, &mut temp_index, out)?;
            }
            IrOp::SubInt { .. } => {
                emit_closure_binary("-", &mut stack, &mut temp_index, out)?;
            }
            IrOp::MulInt { .. } => {
                emit_closure_binary("*", &mut stack, &mut temp_index, out)?;
            }
            IrOp::DivInt { .. } => {
                emit_closure_binary("/", &mut stack, &mut temp_index, out)?;
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_stack_value(&mut stack, "cmp_int right operand")?;
                let left = pop_stack_value(&mut stack, "cmp_int left operand")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let operator = match kind {
                    CmpKind::Eq => "==",
                    CmpKind::NotEq => "!=",
                    CmpKind::Lt => "<",
                    CmpKind::Lte => "<=",
                    CmpKind::Gt => ">",
                    CmpKind::Gte => ">=",
                    CmpKind::StrictEq => "==",
                    CmpKind::StrictNotEq => "!=",
                };
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool(({left} {operator} {right}) ? 1 : 0);\n"
                ));
                stack.push(temp);
            }
            IrOp::ToString { .. } => {
                let input = pop_stack_value(&mut stack, "to_string input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_to_string({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Not { .. } => {
                let input = pop_stack_value(&mut stack, "not input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_not({input});\n"));
                stack.push(temp);
            }
            IrOp::Bang { .. } => {
                let input = pop_stack_value(&mut stack, "bang input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_bang({input});\n"));
                stack.push(temp);
            }
            IrOp::Question { .. } => {
                let input = pop_stack_value(&mut stack, "question input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_question({input});\n"));
                stack.push(temp);
            }
            IrOp::Raise { .. } => {
                let input = pop_stack_value(&mut stack, "raise input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_raise({input});\n"));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                emit_closure_call(callee, *argc, &mut stack, &mut temp_index, out)?;
            }
            IrOp::CallValue { argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(&mut stack, "closure argument")?);
                }
                args.reverse();
                let callee = pop_stack_value(&mut stack, "closure callee")?;

                let root_frame = format!("root_frame_{temp_index}");
                out.push_str(&format!(
                    "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({callee});\n"));
                for argument in &args {
                    out.push_str(&format!("  tn_runtime_root_register({argument});\n"));
                }

                let call_args = std::iter::once(callee)
                    .chain(std::iter::once(format!("(TnVal){argc}")))
                    .chain(args.into_iter())
                    .collect::<Vec<_>>()
                    .join(", ");

                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_call_closure_varargs({call_args});\n"
                ));
                out.push_str(&format!("  tn_runtime_retain({temp});\n"));
                out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                out.push_str(&format!("  tn_runtime_release({temp});\n"));
                stack.push(temp);
            }
            IrOp::Return { .. } => {
                let value = pop_stack_value(&mut stack, "return value")?;
                out.push_str(&format!("  tn_runtime_retain({value});\n"));
                out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
                out.push_str(
                    "  tn_binding_restore(tn_closure_bindings, tn_closure_bindings_len);\n",
                );
                out.push_str(&format!("  return {value};\n"));
                emitted_return = true;
                break;
            }
            _ => {
                out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
                out.push_str(
                    "  tn_binding_restore(tn_closure_bindings, tn_closure_bindings_len);\n",
                );
                out.push_str(
                    "  return tn_runtime_fail(\"unsupported closure operation in native runtime\");\n",
                );
                emitted_return = true;
                break;
            }
        }
    }

    if !emitted_return {
        if let Some(value) = stack.pop() {
            out.push_str(&format!("  tn_runtime_retain({value});\n"));
            out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
            out.push_str("  tn_binding_restore(tn_closure_bindings, tn_closure_bindings_len);\n");
            out.push_str(&format!("  return {value};\n"));
        } else {
            out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
            out.push_str("  tn_binding_restore(tn_closure_bindings, tn_closure_bindings_len);\n");
            out.push_str(
                "  return tn_runtime_fail(\"anonymous function ended without return\");\n",
            );
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn emit_closure_binary(
    operator: &str,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let right = pop_stack_value(stack, "binary right operand")?;
    let left = pop_stack_value(stack, "binary left operand")?;
    let temp = format!("tmp_{}", *temp_index);
    *temp_index += 1;
    out.push_str(&format!("  TnVal {temp} = {left} {operator} {right};\n"));
    stack.push(temp);
    Ok(())
}

fn emit_closure_call(
    callee: &IrCallTarget,
    argc: usize,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_stack_value(stack, "call argument")?);
    }
    args.reverse();

    let rendered_args = args.join(", ");
    let temp = format!("tmp_{}", *temp_index);
    let root_frame = format!("root_frame_{}", *temp_index);
    *temp_index += 1;

    out.push_str(&format!(
        "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
    ));
    for argument in &args {
        out.push_str(&format!("  tn_runtime_root_register({argument});\n"));
    }

    match callee {
        IrCallTarget::Builtin { name } => match name.as_str() {
            "tuple" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_tuple({rendered_args});\n"
                ));
            }
            "list" => {
                let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_list_varargs({count_then_args});\n"
                ));
            }
            "map_empty" => {
                out.push_str(&format!("  TnVal {temp} = tn_runtime_map_empty();\n"));
            }
            "map" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_map({rendered_args});\n"
                ));
            }
            "map_put" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_put({rendered_args});\n"
                ));
            }
            "map_update" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_update({rendered_args});\n"
                ));
            }
            "map_access" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_access({rendered_args});\n"
                ));
            }
            "keyword" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_keyword({rendered_args});\n"
                ));
            }
            "keyword_append" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_keyword_append({rendered_args});\n"
                ));
            }
            "host_call" => {
                let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_host_call_varargs({count_then_args});\n"
                ));
            }
            "protocol_dispatch" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_protocol_dispatch({rendered_args});\n"
                ));
            }
            "ok" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_ok({rendered_args});\n"
                ));
            }
            "err" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_err({rendered_args});\n"
                ));
            }
            other => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_fail(\"unsupported closure builtin call target: {other}\");\n"
                ));
            }
        },
        IrCallTarget::Function { name } => {
            let symbol = mangle_function_name(name, argc);
            out.push_str(&format!("  TnVal {temp} = {symbol}({rendered_args});\n"));
        }
    }

    out.push_str(&format!("  tn_runtime_retain({temp});\n"));
    out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
    out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
    out.push_str(&format!("  tn_runtime_release({temp});\n"));
    stack.push(temp);
    Ok(())
}
