use std::collections::BTreeMap;

use crate::guard_builtins;
use crate::ir::{CmpKind, IrCallTarget, IrCaseBranch, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::{
    closure_capture_names, hash_closure_descriptor_i64, hash_pattern_i64, hash_text_i64,
};
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
    let emitted_return = emit_closure_ops(&closure.ops, &params, &mut stack, &mut temp_index, out)?;

    if !emitted_return {
        if let Some(value) = stack.pop() {
            emit_closure_return_value(&value, out);
        } else {
            emit_closure_cleanup(out);
            out.push_str(
                "  return tn_runtime_fail(\"anonymous function ended without return\");\n",
            );
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn emit_closure_ops(
    ops: &[IrOp],
    params: &BTreeMap<String, usize>,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<bool, CBackendError> {
    for op in ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(position) = params.get(name) {
                    stack.push(format!("argv[{position}]"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    let temp = format!("tmp_{temp_index}");
                    *temp_index += 1;
                    out.push_str(&format!(
                        "  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(temp);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_const_nil();\n"));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::AddInt { .. } => {
                emit_closure_binary("+", stack, temp_index, out)?;
            }
            IrOp::SubInt { .. } => {
                emit_closure_binary("-", stack, temp_index, out)?;
            }
            IrOp::MulInt { .. } => {
                emit_closure_binary("*", stack, temp_index, out)?;
            }
            IrOp::DivInt { .. } => {
                emit_closure_binary("/", stack, temp_index, out)?;
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_stack_value(stack, "cmp_int right operand")?;
                let left = pop_stack_value(stack, "cmp_int left operand")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                match kind {
                    CmpKind::Eq | CmpKind::StrictEq => {
                        out.push_str(&format!(
                            "  TnVal {temp} = tn_runtime_const_bool(tn_runtime_value_equal({left}, {right}) ? 1 : 0);\n"
                        ));
                    }
                    CmpKind::NotEq | CmpKind::StrictNotEq => {
                        out.push_str(&format!(
                            "  TnVal {temp} = tn_runtime_const_bool(tn_runtime_value_equal({left}, {right}) ? 0 : 1);\n"
                        ));
                    }
                    CmpKind::Lt | CmpKind::Lte | CmpKind::Gt | CmpKind::Gte => {
                        let operator = match kind {
                            CmpKind::Lt => "<",
                            CmpKind::Lte => "<=",
                            CmpKind::Gt => ">",
                            CmpKind::Gte => ">=",
                            _ => unreachable!(),
                        };
                        out.push_str(&format!(
                            "  TnVal {temp} = tn_runtime_const_bool(({left} {operator} {right}) ? 1 : 0);\n"
                        ));
                    }
                }
                stack.push(temp);
            }
            IrOp::ToString { .. } => {
                let input = pop_stack_value(stack, "to_string input")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_to_string({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Not { .. } => {
                let input = pop_stack_value(stack, "not input")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_not({input});\n"));
                stack.push(temp);
            }
            IrOp::Bang { .. } => {
                let input = pop_stack_value(stack, "bang input")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_bang({input});\n"));
                stack.push(temp);
            }
            IrOp::Question { .. } => {
                let input = pop_stack_value(stack, "question input")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_question({input});\n"));
                stack.push(temp);
            }
            IrOp::Raise { .. } => {
                let input = pop_stack_value(stack, "raise input")?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_raise({input});\n"));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                emit_closure_call(callee, *argc, stack, temp_index, out)?;
            }
            IrOp::CallValue { argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(stack, "closure argument")?);
                }
                args.reverse();
                let callee = pop_stack_value(stack, "closure callee")?;

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
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_call_closure_varargs({call_args});\n"
                ));
                out.push_str(&format!("  tn_runtime_retain({temp});\n"));
                out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                out.push_str(&format!("  tn_runtime_release({temp});\n"));
                stack.push(temp);
            }
            IrOp::Match { pattern, .. } => {
                let value = pop_stack_value(stack, "closure match value")?;
                let pattern_hash = hash_pattern_i64(pattern)?;
                let temp = format!("tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_match_operator({value}, (TnVal){pattern_hash}LL);\n"
                ));
                stack.push(temp);
            }
            IrOp::Case { branches, .. } => {
                emit_closure_case(branches, params, stack, temp_index, out)?;
            }
            IrOp::Drop => {
                stack.pop();
            }
            IrOp::Return { .. } => {
                let value = pop_stack_value(stack, "return value")?;
                emit_closure_return_value(&value, out);
                return Ok(true);
            }
            _ => {
                emit_closure_cleanup(out);
                out.push_str(
                    "  return tn_runtime_fail(\"unsupported closure operation in native runtime\");\n",
                );
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn emit_closure_case(
    branches: &[IrCaseBranch],
    params: &BTreeMap<String, usize>,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let subject = pop_stack_value(stack, "closure case subject")?;
    let case_index = *temp_index;
    let case_result = format!("tmp_{case_index}");
    *temp_index += 1;
    let case_matched = format!("case_matched_{case_index}");
    let case_bindings = format!("tn_case_bindings_{case_index}");
    let case_bindings_len = format!("tn_case_bindings_len_{case_index}");

    out.push_str(&format!(
        "  TnVal {case_result} = tn_runtime_const_nil();\n"
    ));
    out.push_str(&format!("  int {case_matched} = 0;\n"));
    out.push_str(&format!("  TnBinding {case_bindings}[TN_MAX_BINDINGS];\n"));
    out.push_str(&format!("  size_t {case_bindings_len} = 0;\n"));
    out.push_str(&format!(
        "  tn_binding_snapshot({case_bindings}, &{case_bindings_len});\n"
    ));

    for (branch_index, branch) in branches.iter().enumerate() {
        let condition = if matches!(branch.pattern, IrPattern::Wildcard) {
            "1".to_string()
        } else {
            let pattern_hash = hash_pattern_i64(&branch.pattern)?;
            format!("tn_runtime_pattern_matches({subject}, (TnVal){pattern_hash}LL)")
        };

        out.push_str(&format!(
            "  if (!{case_matched}) {{\n    tn_binding_restore({case_bindings}, {case_bindings_len});\n    if ({condition}) {{\n"
        ));

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_expr = emit_closure_guard_ops(
                guard_ops,
                params,
                temp_index,
                out,
                &format!("case_{case_index}_guard_{branch_index}"),
            )?;
            out.push_str(&format!(
                "      if (tn_runtime_is_truthy({guard_expr})) {{\n"
            ));
        }

        let mut branch_stack = Vec::new();
        let returned = emit_closure_ops(&branch.ops, params, &mut branch_stack, temp_index, out)?;
        if !returned {
            let branch_value = branch_stack
                .pop()
                .unwrap_or_else(|| "tn_runtime_const_nil()".to_string());
            out.push_str(&format!("        {case_result} = {branch_value};\n"));
            out.push_str(&format!("        {case_matched} = 1;\n"));
            out.push_str(&format!(
                "        tn_binding_restore({case_bindings}, {case_bindings_len});\n"
            ));
        }

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n  }\n");
    }

    out.push_str(&format!(
        "  tn_binding_restore({case_bindings}, {case_bindings_len});\n"
    ));
    out.push_str(&format!("  if (!{case_matched}) {{\n"));
    emit_closure_cleanup(out);
    out.push_str("    return tn_runtime_fail(\"no case clause matching\");\n");
    out.push_str("  }\n");

    stack.push(case_result);
    Ok(())
}

fn emit_closure_guard_ops(
    guard_ops: &[IrOp],
    params: &BTreeMap<String, usize>,
    temp_index: &mut usize,
    out: &mut String,
    label: &str,
) -> Result<String, CBackendError> {
    let mut stack = Vec::<String>::new();

    for op in guard_ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(position) = params.get(name) {
                    stack.push(format!("argv[{position}]"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    let temp = format!("{label}_tmp_{temp_index}");
                    *temp_index += 1;
                    out.push_str(&format!(
                        "  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(temp);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_const_nil();\n"));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_stack_value(&mut stack, "closure guard cmp right operand")?;
                let left = pop_stack_value(&mut stack, "closure guard cmp left operand")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                match kind {
                    CmpKind::Eq | CmpKind::StrictEq => {
                        out.push_str(&format!(
                            "  TnVal {temp} = tn_runtime_value_equal({left}, {right}) ? 1 : 0;\n"
                        ));
                    }
                    CmpKind::NotEq | CmpKind::StrictNotEq => {
                        out.push_str(&format!(
                            "  TnVal {temp} = tn_runtime_value_equal({left}, {right}) ? 0 : 1;\n"
                        ));
                    }
                    CmpKind::Lt | CmpKind::Lte | CmpKind::Gt | CmpKind::Gte => {
                        let operator = match kind {
                            CmpKind::Lt => "<",
                            CmpKind::Lte => "<=",
                            CmpKind::Gt => ">",
                            CmpKind::Gte => ">=",
                            _ => unreachable!(),
                        };
                        out.push_str(&format!(
                            "  TnVal {temp} = ({left} {operator} {right}) ? 1 : 0;\n"
                        ));
                    }
                }
                stack.push(temp);
            }
            IrOp::Bang { .. } => {
                let input = pop_stack_value(&mut stack, "closure guard bang input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_bang({input});\n"));
                stack.push(temp);
            }
            IrOp::Not { .. } => {
                let input = pop_stack_value(&mut stack, "closure guard not input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_not({input});\n"));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(&mut stack, "closure guard call argument")?);
                }
                args.reverse();
                let rendered_args = args.join(", ");
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;

                match callee {
                    IrCallTarget::Builtin { name } => {
                        if let Some(helper) = guard_builtins::c_helper_name(name) {
                            if args.len() != guard_builtins::GUARD_BUILTIN_ARITY {
                                return Err(CBackendError::new(format!(
                                    "c backend closure guard builtin {name} arity mismatch"
                                )));
                            }

                            out.push_str(&format!("  TnVal {temp} = {helper}({rendered_args});\n"));
                        } else {
                            return Err(CBackendError::new(format!(
                                "c backend closure guard unsupported builtin call target: {name}"
                            )));
                        }
                    }
                    IrCallTarget::Function { name } => {
                        let root_frame = format!("{label}_rf_{temp_index}");
                        let symbol = mangle_function_name(name, *argc);
                        out.push_str(&format!(
                            "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                        ));
                        for argument in &args {
                            out.push_str(&format!("  tn_runtime_root_register({argument});\n"));
                        }
                        out.push_str(&format!("  TnVal {temp} = {symbol}({rendered_args});\n"));
                        out.push_str(&format!("  tn_runtime_retain({temp});\n"));
                        out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
                        out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                        out.push_str(&format!("  tn_runtime_release({temp});\n"));
                    }
                }

                stack.push(temp);
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend unsupported closure guard op: {other:?}"
                )));
            }
        }
    }

    stack
        .pop()
        .ok_or_else(|| CBackendError::new("c backend empty closure guard stack"))
}

fn emit_closure_cleanup(out: &mut String) {
    out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
    out.push_str("  tn_binding_restore(tn_closure_bindings, tn_closure_bindings_len);\n");
}

fn emit_closure_return_value(value: &str, out: &mut String) {
    out.push_str(&format!("  tn_runtime_retain({value});\n"));
    emit_closure_cleanup(out);
    out.push_str(&format!("  return {value};\n"));
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
