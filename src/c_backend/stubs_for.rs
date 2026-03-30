use std::collections::BTreeMap;

use crate::backend_names::mangle_function_name;
use crate::cli_diag::failure_message_lines_with_filename_and_source;
use crate::ir::{CmpKind, IrCallTarget, IrCaseBranch, IrForGenerator, IrOp};
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::{hash_ir_op_i64, hash_pattern_i64, hash_text_i64};
use super::stubs::c_string_literal;

#[path = "stubs_for_ops.rs"]
mod ops;
use ops::{apply_pattern_bindings, evaluate_static_for_ops};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ForSpec {
    pub(super) hash: i64,
    pub(super) op: IrOp,
}

const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";

#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticForValue {
    Int(i64),
    Bool(bool),
    Nil,
    Atom(String),
    String(String),
    Float(String),
    Range(i64, i64),
    SteppedRange(i64, i64, i64),
    Tuple(Box<StaticForValue>, Box<StaticForValue>),
    List(Vec<StaticForValue>),
    Map(Vec<(StaticForValue, StaticForValue)>),
    Keyword(Vec<(StaticForValue, StaticForValue)>),
}

impl StaticForValue {
    fn kind_label(&self) -> &'static str {
        match self {
            StaticForValue::Int(_) => "int",
            StaticForValue::Bool(_) => "bool",
            StaticForValue::Nil => "nil",
            StaticForValue::Atom(_) => "atom",
            StaticForValue::String(_) => "string",
            StaticForValue::Float(_) => "float",
            StaticForValue::Range(_, _) => "range",
            StaticForValue::SteppedRange(_, _, _) => "stepped_range",
            StaticForValue::Tuple(_, _) => "tuple",
            StaticForValue::List(_) => "list",
            StaticForValue::Map(_) => "map",
            StaticForValue::Keyword(_) => "keyword",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticForEvalIssue {
    Runtime(String),
    Unsupported(String),
}

pub(super) fn emit_runtime_for_helpers(
    mir: &MirProgram,
    source_path: &str,
    source: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    let for_specs = collect_for_specs(mir)?;

    out.push_str("/* compiled for helpers */\n");
    for (index, for_spec) in for_specs.iter().enumerate() {
        emit_runtime_for_case(index, for_spec, source_path, source, out)?;
    }

    out.push_str("static TnVal tn_runtime_for(TnVal op_hash) {\n");
    if for_specs.is_empty() {
        out.push_str("  return tn_stub_abort(\"tn_runtime_for\");\n");
    } else {
        out.push_str("  switch (op_hash) {\n");
        for (index, for_spec) in for_specs.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_runtime_for_case_{index}();\n",
                for_spec.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str("      return tn_stub_abort(\"tn_runtime_for\");\n");
        out.push_str("  }\n");
    }
    out.push_str("}\n\n");

    Ok(())
}

fn collect_for_specs(mir: &MirProgram) -> Result<Vec<ForSpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, IrOp>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::Legacy { source, .. } = instruction else {
                    continue;
                };

                if !matches!(source, IrOp::For { .. }) {
                    continue;
                }

                let hash = hash_ir_op_i64(source)?;
                if let Some(existing) = by_hash.get(&hash) {
                    if existing != source {
                        return Err(CBackendError::new(format!(
                            "c backend for hash collision for hash {hash}"
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
        .map(|(hash, op)| ForSpec { hash, op })
        .collect())
}

fn emit_runtime_for_case(
    index: usize,
    for_spec: &ForSpec,
    source_path: &str,
    source: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "static TnVal tn_runtime_for_case_{index}(void) {{\n"
    ));
    out.push_str("  TnBinding tn_for_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t tn_for_bindings_len = 0;\n");
    out.push_str("  tn_binding_snapshot(tn_for_bindings, &tn_for_bindings_len);\n");

    match evaluate_for_spec(&for_spec.op) {
        Ok(value) => {
            let mut temp_index = 0usize;
            let rendered = emit_static_for_value(&value, out, &mut temp_index);
            out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
            out.push_str(&format!("  return {rendered};\n"));
        }
        Err(StaticForEvalIssue::Runtime(message)) => {
            let rendered =
                render_static_for_runtime_failure(source_path, source, for_spec, &message);
            let escaped = c_string_literal(&rendered);
            out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
            out.push_str(&format!("  return tn_runtime_fail({escaped});\n"));
        }
        Err(StaticForEvalIssue::Unsupported(_)) => {
            if !emit_dynamic_for_case_body(for_spec, out)? {
                out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
                out.push_str("  return tn_stub_abort(\"tn_runtime_for\");\n");
            }
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn render_static_for_runtime_failure(
    source_path: &str,
    source: &str,
    for_spec: &ForSpec,
    message: &str,
) -> String {
    let offset = match &for_spec.op {
        IrOp::For { offset, .. } => Some(*offset),
        _ => None,
    };
    failure_message_lines_with_filename_and_source(message, Some(source_path), source, offset)
        .join("\n")
}

fn emit_dynamic_for_case_body(for_spec: &ForSpec, out: &mut String) -> Result<bool, CBackendError> {
    let IrOp::For {
        generators,
        into_ops,
        reduce_ops,
        body_ops,
        ..
    } = &for_spec.op
    else {
        return Ok(false);
    };

    if into_ops.is_some() || generators.len() != 1 {
        return Ok(false);
    }

    let generator = &generators[0];
    if !supports_dynamic_for_ops(&generator.source_ops)
        || generator
            .guard_ops
            .as_ref()
            .is_some_and(|ops| !supports_dynamic_for_ops(ops))
        || !supports_dynamic_for_ops(body_ops)
        || reduce_ops
            .as_ref()
            .is_some_and(|ops| !supports_dynamic_for_ops(ops))
    {
        return Ok(false);
    }

    let pattern_hash = hash_pattern_i64(&generator.pattern)?;
    let mut temp_index = 0usize;

    out.push_str("  size_t tn_for_root_frame = tn_runtime_root_frame_push();\n");

    let source_value =
        emit_dynamic_for_ops(&generator.source_ops, "tn_for_source", &mut temp_index, out)?;
    out.push_str(&format!("  TnVal tn_for_source = {source_value};\n"));
    out.push_str("  tn_runtime_root_register(tn_for_source);\n");

    if let Some(reduce_ops) = reduce_ops {
        let acc_init =
            emit_dynamic_for_ops(reduce_ops, "tn_for_reduce_init", &mut temp_index, out)?;
        out.push_str(&format!("  TnVal tn_for_acc = {acc_init};\n"));
        out.push_str("  tn_runtime_root_register(tn_for_acc);\n");
    } else {
        out.push_str("  size_t tn_for_list_len = 0;\n");
        out.push_str("  size_t tn_for_list_cap = 0;\n");
        out.push_str("  TnVal *tn_for_list_items = NULL;\n");
    }

    out.push_str("  TnObj *tn_for_source_obj = tn_get_obj(tn_for_source);\n");
    out.push_str("  if (tn_for_source_obj != NULL && tn_for_source_obj->kind == TN_OBJ_LIST) {\n");
    out.push_str("    for (size_t tn_for_index = 0; tn_for_index < tn_for_source_obj->as.list.len; tn_for_index += 1) {\n");
    out.push_str("      TnVal tn_for_item = tn_for_source_obj->as.list.items[tn_for_index];\n");
    emit_dynamic_for_iteration(
        generator,
        body_ops,
        reduce_ops.as_deref(),
        pattern_hash,
        &mut temp_index,
        "      ",
        out,
    )?;
    out.push_str("    }\n");
    out.push_str(
        "  } else if (tn_for_source_obj != NULL && tn_for_source_obj->kind == TN_OBJ_RANGE) {\n",
    );
    out.push_str("    TnVal tn_for_start = tn_for_source_obj->as.range.start;\n");
    out.push_str("    TnVal tn_for_end = tn_for_source_obj->as.range.end;\n");
    out.push_str("    if (tn_for_start <= tn_for_end) {\n");
    out.push_str("      for (TnVal tn_for_item = tn_for_start; tn_for_item <= tn_for_end; tn_for_item += 1) {\n");
    emit_dynamic_for_iteration(
        generator,
        body_ops,
        reduce_ops.as_deref(),
        pattern_hash,
        &mut temp_index,
        "        ",
        out,
    )?;
    out.push_str("      }\n");
    out.push_str("    }\n");
    out.push_str("  } else {\n");
    out.push_str("    return tn_runtime_failf(\"for requires iterable, found %s\", tn_runtime_value_kind(tn_for_source));\n");
    out.push_str("  }\n");

    if reduce_ops.is_some() {
        out.push_str("  tn_runtime_retain(tn_for_acc);\n");
        out.push_str("  tn_runtime_root_frame_pop(tn_for_root_frame);\n");
        out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
        out.push_str("  return tn_for_acc;\n");
    } else {
        out.push_str("  TnObj *tn_for_result_obj = tn_new_obj(TN_OBJ_LIST);\n");
        out.push_str("  tn_for_result_obj->as.list.len = tn_for_list_len;\n");
        out.push_str("  tn_for_result_obj->as.list.items = tn_for_list_items;\n");
        out.push_str("  for (size_t tn_for_i = 0; tn_for_i < tn_for_list_len; tn_for_i += 1) {\n");
        out.push_str("    tn_runtime_retain(tn_for_result_obj->as.list.items[tn_for_i]);\n");
        out.push_str("  }\n");
        out.push_str("  TnVal tn_for_result = tn_heap_store(tn_for_result_obj);\n");
        out.push_str("  tn_runtime_retain(tn_for_result);\n");
        out.push_str("  tn_runtime_root_frame_pop(tn_for_root_frame);\n");
        out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
        out.push_str("  return tn_for_result;\n");
    }

    Ok(true)
}

fn emit_dynamic_for_iteration(
    generator: &IrForGenerator,
    body_ops: &[IrOp],
    reduce_ops: Option<&[IrOp]>,
    pattern_hash: i64,
    temp_index: &mut usize,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "{indent}TnBinding tn_for_iter_bindings[TN_MAX_BINDINGS];\n"
    ));
    out.push_str(&format!("{indent}size_t tn_for_iter_bindings_len = 0;\n"));
    out.push_str(&format!(
        "{indent}tn_binding_snapshot(tn_for_iter_bindings, &tn_for_iter_bindings_len);\n"
    ));
    out.push_str(&format!(
        "{indent}if (tn_runtime_pattern_matches(tn_for_item, (TnVal){pattern_hash}LL)) {{\n"
    ));

    if let Some(guard_ops) = &generator.guard_ops {
        let guard_value = emit_dynamic_for_ops(guard_ops, "tn_for_guard", temp_index, out)?;
        out.push_str(&format!(
            "{indent}  if (tn_runtime_is_truthy({guard_value})) {{\n"
        ));
    }

    if reduce_ops.is_some() {
        let acc_hash = hash_text_i64(FOR_REDUCE_ACC_BINDING);
        out.push_str(&format!(
            "{indent}    tn_binding_set((TnVal){acc_hash}LL, tn_for_acc);\n"
        ));
        let body_value = emit_dynamic_for_ops(body_ops, "tn_for_reduce_body", temp_index, out)?;
        out.push_str(&format!("{indent}    tn_for_acc = {body_value};\n"));
        out.push_str(&format!(
            "{indent}    tn_runtime_root_register(tn_for_acc);\n"
        ));
    } else {
        let body_value = emit_dynamic_for_ops(body_ops, "tn_for_body", temp_index, out)?;
        emit_dynamic_for_list_append(&body_value, indent, out);
    }

    if generator.guard_ops.is_some() {
        out.push_str(&format!("{indent}  }}\n"));
    }

    out.push_str(&format!("{indent}}}\n"));
    out.push_str(&format!(
        "{indent}tn_binding_restore(tn_for_iter_bindings, tn_for_iter_bindings_len);\n"
    ));
    Ok(())
}

fn emit_dynamic_for_list_append(value: &str, indent: &str, out: &mut String) {
    out.push_str(&format!(
        "{indent}    if (tn_for_list_len == tn_for_list_cap) {{\n"
    ));
    out.push_str(&format!(
        "{indent}      size_t tn_for_next_cap = tn_for_list_cap == 0 ? 4 : tn_for_list_cap * 2;\n"
    ));
    out.push_str(&format!(
        "{indent}      TnVal *tn_for_next_items = (TnVal *)realloc(tn_for_list_items, tn_for_next_cap * sizeof(TnVal));\n"
    ));
    out.push_str(&format!(
        "{indent}      if (tn_for_next_items == NULL) {{\n"
    ));
    out.push_str(&format!(
        "{indent}        fprintf(stderr, \"error: native runtime allocation failure\\n\");\n"
    ));
    out.push_str(&format!("{indent}        exit(1);\n"));
    out.push_str(&format!("{indent}      }}\n"));
    out.push_str(&format!(
        "{indent}      tn_for_list_items = tn_for_next_items;\n"
    ));
    out.push_str(&format!(
        "{indent}      tn_for_list_cap = tn_for_next_cap;\n"
    ));
    out.push_str(&format!("{indent}    }}\n"));
    out.push_str(&format!(
        "{indent}    tn_for_list_items[tn_for_list_len] = {value};\n"
    ));
    out.push_str(&format!("{indent}    tn_runtime_root_register({value});\n"));
    out.push_str(&format!("{indent}    tn_for_list_len += 1;\n"));
}

fn supports_dynamic_for_ops(ops: &[IrOp]) -> bool {
    ops.iter().all(supports_dynamic_for_op)
}

fn supports_dynamic_for_op(op: &IrOp) -> bool {
    match op {
        IrOp::ConstInt { .. }
        | IrOp::ConstBool { .. }
        | IrOp::ConstNil { .. }
        | IrOp::ConstString { .. }
        | IrOp::ConstAtom { .. }
        | IrOp::ConstFloat { .. }
        | IrOp::LoadVariable { .. }
        | IrOp::AddInt { .. }
        | IrOp::SubInt { .. }
        | IrOp::MulInt { .. }
        | IrOp::DivInt { .. }
        | IrOp::Range { .. }
        | IrOp::SteppedRange { .. }
        | IrOp::CallValue { .. }
        | IrOp::Drop
        | IrOp::Return { .. } => true,
        IrOp::CmpInt { kind, .. } => matches!(
            kind,
            CmpKind::Eq
                | CmpKind::NotEq
                | CmpKind::Lt
                | CmpKind::Lte
                | CmpKind::Gt
                | CmpKind::Gte
                | CmpKind::StrictEq
                | CmpKind::StrictNotEq
        ),
        IrOp::Call { callee, .. } => supports_dynamic_for_call_target(callee),
        IrOp::Case { branches, .. } => branches.iter().all(|branch| {
            branch
                .guard_ops
                .as_ref()
                .is_none_or(|ops| supports_dynamic_for_ops(ops))
                && supports_dynamic_for_ops(&branch.ops)
        }),
        _ => false,
    }
}

fn supports_dynamic_for_call_target(callee: &IrCallTarget) -> bool {
    match callee {
        IrCallTarget::Function { .. } => true,
        IrCallTarget::Builtin { name } => matches!(
            name.as_str(),
            "tuple"
                | "list"
                | "map_empty"
                | "map"
                | "map_put"
                | "keyword"
                | "keyword_append"
                | "ok"
                | "err"
                | "to_string"
                | "protocol_dispatch"
                | "div"
                | "rem"
                | "byte_size"
                | "bit_size"
                | "hd"
                | "tl"
                | "elem"
                | "tuple_size"
                | "put_elem"
        ),
    }
}

fn emit_dynamic_for_ops(
    ops: &[IrOp],
    label: &str,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<String, CBackendError> {
    let mut stack = Vec::<String>::new();

    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = (TnVal){value}LL;\n"));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_const_nil();\n"));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::LoadVariable { name, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let binding_hash = hash_text_i64(name);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::AddInt { .. } => {
                emit_dynamic_for_binary("tn_runtime_arith_add", label, temp_index, &mut stack, out)?
            }
            IrOp::SubInt { .. } => {
                emit_dynamic_for_binary("tn_runtime_arith_sub", label, temp_index, &mut stack, out)?
            }
            IrOp::MulInt { .. } => {
                emit_dynamic_for_binary("tn_runtime_arith_mul", label, temp_index, &mut stack, out)?
            }
            IrOp::DivInt { .. } => {
                emit_dynamic_for_binary("tn_runtime_arith_div", label, temp_index, &mut stack, out)?
            }
            IrOp::Range { .. } => {
                let right = pop_dynamic_for_value(&mut stack, "range right")?;
                let left = pop_dynamic_for_value(&mut stack, "range left")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_range({left}, {right});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::SteppedRange { .. } => {
                let step = pop_dynamic_for_value(&mut stack, "stepped range step")?;
                let range = pop_dynamic_for_value(&mut stack, "stepped range range")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_stepped_range({range}, {step});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_dynamic_for_value(&mut stack, "cmp right")?;
                let left = pop_dynamic_for_value(&mut stack, "cmp left")?;
                let temp = format!("{label}_tmp_{temp_index}");
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
                        let helper = match kind {
                            CmpKind::Lt => "tn_runtime_cmp_lt",
                            CmpKind::Lte => "tn_runtime_cmp_lte",
                            CmpKind::Gt => "tn_runtime_cmp_gt",
                            CmpKind::Gte => "tn_runtime_cmp_gte",
                            _ => unreachable!(),
                        };
                        out.push_str(&format!("  TnVal {temp} = {helper}({left}, {right});\n"));
                    }
                }
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                emit_dynamic_for_call(callee, *argc, label, temp_index, &mut stack, out)?;
            }
            IrOp::CallValue { argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_dynamic_for_value(&mut stack, "call value argument")?);
                }
                args.reverse();
                let callee = pop_dynamic_for_value(&mut stack, "call value callee")?;
                let temp = format!("{label}_tmp_{temp_index}");
                *temp_index += 1;
                let rendered_args = std::iter::once(callee)
                    .chain(std::iter::once(format!("(TnVal){argc}")))
                    .chain(args.into_iter())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_call_closure_varargs({rendered_args});\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                stack.push(temp);
            }
            IrOp::Case { branches, .. } => {
                emit_dynamic_for_case_expr(branches, label, temp_index, &mut stack, out)?;
            }
            IrOp::Drop => {
                stack.pop();
            }
            IrOp::Return { .. } => {
                return Ok(stack
                    .pop()
                    .unwrap_or_else(|| "tn_runtime_const_nil()".to_string()));
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend unsupported dynamic for op: {other:?}"
                )));
            }
        }
    }

    Ok(stack
        .pop()
        .unwrap_or_else(|| "tn_runtime_const_nil()".to_string()))
}

fn emit_dynamic_for_case_expr(
    branches: &[IrCaseBranch],
    label: &str,
    temp_index: &mut usize,
    stack: &mut Vec<String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let subject = pop_dynamic_for_value(stack, "case subject")?;
    let case_index = *temp_index;
    let case_result = format!("{label}_case_{case_index}");
    let case_matched = format!("{label}_case_matched_{case_index}");
    let case_bindings = format!("{label}_case_bindings_{case_index}");
    let case_bindings_len = format!("{label}_case_bindings_len_{case_index}");
    *temp_index += 1;

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
        let condition = if matches!(branch.pattern, crate::ir::IrPattern::Wildcard) {
            "1".to_string()
        } else {
            let pattern_hash = hash_pattern_i64(&branch.pattern)?;
            format!("tn_runtime_pattern_matches({subject}, (TnVal){pattern_hash}LL)")
        };

        out.push_str(&format!(
            "  if (!{case_matched}) {{\n    tn_binding_restore({case_bindings}, {case_bindings_len});\n    if ({condition}) {{\n"
        ));

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_value = emit_dynamic_for_ops(
                guard_ops,
                &format!("{label}_case_guard_{branch_index}"),
                temp_index,
                out,
            )?;
            out.push_str(&format!(
                "      if (tn_runtime_is_truthy({guard_value})) {{\n"
            ));
        }

        let branch_value = emit_dynamic_for_ops(
            &branch.ops,
            &format!("{label}_case_body_{branch_index}"),
            temp_index,
            out,
        )?;
        out.push_str(&format!("        {case_result} = {branch_value};\n"));
        out.push_str(&format!("        {case_matched} = 1;\n"));

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n  }\n");
    }

    out.push_str(&format!(
        "  tn_binding_restore({case_bindings}, {case_bindings_len});\n"
    ));
    out.push_str(&format!("  if (!{case_matched}) {{\n"));
    out.push_str("    return tn_runtime_fail(\"no case clause matching\");\n");
    out.push_str("  }\n");
    out.push_str(&format!("  tn_runtime_root_register({case_result});\n"));
    stack.push(case_result);
    Ok(())
}

fn emit_dynamic_for_binary(
    helper: &str,
    label: &str,
    temp_index: &mut usize,
    stack: &mut Vec<String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let right = pop_dynamic_for_value(stack, "binary right operand")?;
    let left = pop_dynamic_for_value(stack, "binary left operand")?;
    let temp = format!("{label}_tmp_{temp_index}");
    *temp_index += 1;
    out.push_str(&format!("  TnVal {temp} = {helper}({left}, {right});\n"));
    out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
    stack.push(temp);
    Ok(())
}

fn emit_dynamic_for_call(
    callee: &IrCallTarget,
    argc: usize,
    label: &str,
    temp_index: &mut usize,
    stack: &mut Vec<String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_dynamic_for_value(stack, "call argument")?);
    }
    args.reverse();
    let rendered_args = args.join(", ");

    let temp = format!("{label}_tmp_{temp_index}");
    *temp_index += 1;

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
            "to_string" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_to_string({rendered_args});\n"
                ));
            }
            "protocol_dispatch" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_protocol_dispatch({rendered_args});\n"
                ));
            }
            "div" => {
                let args: Vec<&str> = rendered_args.split(", ").collect();
                if args.len() != 2 {
                    return Err(CBackendError::new(
                        "c backend dynamic for builtin div arity mismatch",
                    ));
                }
                out.push_str(&format!(
                    "  TnVal {temp} = (TnVal)({} / {});\n",
                    args[0], args[1]
                ));
            }
            "rem" => {
                let args: Vec<&str> = rendered_args.split(", ").collect();
                if args.len() != 2 {
                    return Err(CBackendError::new(
                        "c backend dynamic for builtin rem arity mismatch",
                    ));
                }
                out.push_str(&format!(
                    "  TnVal {temp} = (TnVal)({} % {});\n",
                    args[0], args[1]
                ));
            }
            "byte_size" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_byte_size({rendered_args});\n"
                ));
            }
            "bit_size" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_bit_size({rendered_args});\n"
                ));
            }
            "hd" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_hd({rendered_args});\n"
                ));
            }
            "tl" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_tl({rendered_args});\n"
                ));
            }
            "elem" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_elem({rendered_args});\n"
                ));
            }
            "tuple_size" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_tuple_size({rendered_args});\n"
                ));
            }
            "put_elem" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_put_elem({rendered_args});\n"
                ));
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend unsupported dynamic for builtin call target: {other}"
                )));
            }
        },
        IrCallTarget::Function { name } => {
            let binding_snapshot = format!("{label}_call_bindings_{temp_index}");
            let binding_snapshot_len = format!("{label}_call_bindings_len_{temp_index}");
            let symbol = mangle_function_name(name, argc);
            out.push_str(&format!(
                "  TnBinding {binding_snapshot}[TN_MAX_BINDINGS];\n"
            ));
            out.push_str(&format!("  size_t {binding_snapshot_len} = 0;\n"));
            out.push_str(&format!(
                "  tn_binding_snapshot({binding_snapshot}, &{binding_snapshot_len});\n"
            ));
            out.push_str(&format!("  tn_binding_restore({binding_snapshot}, 0);\n"));
            out.push_str(&format!("  TnVal {temp} = {symbol}({rendered_args});\n"));
            out.push_str(&format!(
                "  tn_binding_restore({binding_snapshot}, {binding_snapshot_len});\n"
            ));
        }
    }

    out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
    stack.push(temp);
    Ok(())
}

fn pop_dynamic_for_value(stack: &mut Vec<String>, context: &str) -> Result<String, CBackendError> {
    stack.pop().ok_or_else(|| {
        CBackendError::new(format!(
            "c backend dynamic for stack underflow for {context}"
        ))
    })
}

fn emit_static_for_value(
    value: &StaticForValue,
    out: &mut String,
    temp_index: &mut usize,
) -> String {
    match value {
        StaticForValue::Int(value) => format!("(TnVal){value}LL"),
        StaticForValue::Bool(value) => {
            format!(
                "tn_runtime_const_bool((TnVal){})",
                if *value { 1 } else { 0 }
            )
        }
        StaticForValue::Nil => "tn_runtime_const_nil()".to_string(),
        StaticForValue::Atom(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_atom((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::String(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_string((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::Float(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_float((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::Range(start, end) => {
            format!("tn_runtime_range((TnVal){start}LL, (TnVal){end}LL)")
        }
        StaticForValue::SteppedRange(..) => {
            "tn_stub_abort(\"tn_runtime_stepped_range\")".to_string()
        }
        StaticForValue::Tuple(left, right) => {
            let left_value = emit_static_for_value(left, out, temp_index);
            let right_value = emit_static_for_value(right, out, temp_index);
            format!("tn_runtime_make_tuple({left_value}, {right_value})")
        }
        StaticForValue::List(values) => {
            let rendered_items = values
                .iter()
                .map(|item| emit_static_for_value(item, out, temp_index))
                .collect::<Vec<_>>();
            let args = std::iter::once(format!("(TnVal){}", values.len()))
                .chain(rendered_items)
                .collect::<Vec<_>>()
                .join(", ");
            format!("tn_runtime_make_list_varargs({args})")
        }
        StaticForValue::Map(entries) => {
            if entries.is_empty() {
                "tn_runtime_map_empty()".to_string()
            } else {
                let temp = format!("tn_for_value_{}", *temp_index);
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_map_empty();\n"));
                for (key, value) in entries {
                    let rendered_key = emit_static_for_value(key, out, temp_index);
                    let rendered_value = emit_static_for_value(value, out, temp_index);
                    out.push_str(&format!(
                        "  {temp} = tn_runtime_map_put({temp}, {rendered_key}, {rendered_value});\n"
                    ));
                }
                temp
            }
        }
        StaticForValue::Keyword(entries) => {
            if entries.is_empty() {
                "tn_runtime_make_list_varargs((TnVal)0)".to_string()
            } else {
                let temp = format!("tn_for_value_{}", *temp_index);
                *temp_index += 1;
                let (first_key, first_value) = &entries[0];
                let rendered_first_key = emit_static_for_value(first_key, out, temp_index);
                let rendered_first_value = emit_static_for_value(first_value, out, temp_index);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_keyword({rendered_first_key}, {rendered_first_value});\n"
                ));

                for (key, value) in entries.iter().skip(1) {
                    let rendered_key = emit_static_for_value(key, out, temp_index);
                    let rendered_value = emit_static_for_value(value, out, temp_index);
                    out.push_str(&format!(
                        "  {temp} = tn_runtime_keyword_append({temp}, {rendered_key}, {rendered_value});\n"
                    ));
                }

                temp
            }
        }
    }
}

enum StaticForCollector {
    List(Vec<StaticForValue>),
    Map(Vec<(StaticForValue, StaticForValue)>),
    Keyword(Vec<(StaticForValue, StaticForValue)>),
    Reduce(StaticForValue),
}

fn evaluate_for_spec(for_op: &IrOp) -> Result<StaticForValue, StaticForEvalIssue> {
    let IrOp::For {
        generators,
        into_ops,
        reduce_ops,
        body_ops,
        ..
    } = for_op
    else {
        return Err(StaticForEvalIssue::Unsupported(
            "for helper source was not IrOp::For".to_string(),
        ));
    };

    if into_ops.is_some() && reduce_ops.is_some() {
        return Err(StaticForEvalIssue::Runtime(
            "for options 'reduce' and 'into' cannot be combined".to_string(),
        ));
    }

    let mut collector = if let Some(reduce_ops) = reduce_ops {
        StaticForCollector::Reduce(evaluate_static_for_ops(reduce_ops, &BTreeMap::new())?)
    } else if let Some(into_ops) = into_ops {
        match evaluate_static_for_ops(into_ops, &BTreeMap::new())? {
            StaticForValue::List(values) => StaticForCollector::List(values),
            StaticForValue::Map(entries) => StaticForCollector::Map(entries),
            StaticForValue::Keyword(entries) => StaticForCollector::Keyword(entries),
            other => {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into destination must be a list, map, or keyword, found {}",
                    other.kind_label()
                )));
            }
        }
    } else {
        StaticForCollector::List(Vec::new())
    };

    evaluate_for_generators(generators, 0, &BTreeMap::new(), body_ops, &mut collector)?;

    Ok(match collector {
        StaticForCollector::List(values) => StaticForValue::List(values),
        StaticForCollector::Map(entries) => StaticForValue::Map(entries),
        StaticForCollector::Keyword(entries) => StaticForValue::Keyword(entries),
        StaticForCollector::Reduce(value) => value,
    })
}

fn evaluate_for_generators(
    generators: &[IrForGenerator],
    index: usize,
    env: &BTreeMap<String, StaticForValue>,
    body_ops: &[IrOp],
    collector: &mut StaticForCollector,
) -> Result<(), StaticForEvalIssue> {
    if index >= generators.len() {
        match collector {
            StaticForCollector::Reduce(accumulator) => {
                let mut reduce_env = env.clone();
                reduce_env.insert(FOR_REDUCE_ACC_BINDING.to_string(), accumulator.clone());
                *accumulator = evaluate_static_for_ops(body_ops, &reduce_env)?;
            }
            _ => {
                let body_value = evaluate_static_for_ops(body_ops, env)?;
                collect_for_value(collector, body_value)?;
            }
        }
        return Ok(());
    }

    let generator = &generators[index];
    let enumerable = evaluate_static_for_ops(&generator.source_ops, env)?;
    let values = iter_static_for_source(enumerable)?;

    for value in values {
        let mut iteration_env = env.clone();
        if !apply_pattern_bindings(&generator.pattern, &value, &mut iteration_env)? {
            continue;
        }

        if let Some(guard_ops) = &generator.guard_ops {
            let guard_value = evaluate_static_for_ops(guard_ops, &iteration_env)?;
            let StaticForValue::Bool(guard_result) = guard_value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for generator guard must evaluate to bool, found {}",
                    guard_value.kind_label()
                )));
            };

            if !guard_result {
                continue;
            }
        }

        evaluate_for_generators(generators, index + 1, &iteration_env, body_ops, collector)?;
    }

    Ok(())
}

fn iter_static_for_source(
    source: StaticForValue,
) -> Result<Vec<StaticForValue>, StaticForEvalIssue> {
    match source {
        StaticForValue::List(values) => Ok(values),
        StaticForValue::Range(start, end) => {
            if start <= end {
                Ok((start..=end).map(StaticForValue::Int).collect())
            } else {
                Ok(Vec::new())
            }
        }
        StaticForValue::SteppedRange(start, end, step) => {
            let mut values = Vec::new();
            let mut current = start;
            if step > 0 {
                while current <= end {
                    values.push(StaticForValue::Int(current));
                    current += step;
                }
            } else if step < 0 {
                while current >= end {
                    values.push(StaticForValue::Int(current));
                    current += step;
                }
            }
            Ok(values)
        }
        StaticForValue::Map(entries) => Ok(entries
            .into_iter()
            .map(|(key, value)| StaticForValue::Tuple(Box::new(key), Box::new(value)))
            .collect()),
        other => Err(StaticForEvalIssue::Runtime(format!(
            "for requires iterable, found {}",
            other.kind_label()
        ))),
    }
}

fn collect_for_value(
    collector: &mut StaticForCollector,
    value: StaticForValue,
) -> Result<(), StaticForEvalIssue> {
    match collector {
        StaticForCollector::List(values) => values.push(value),
        StaticForCollector::Map(entries) => {
            let StaticForValue::Tuple(key, entry_value) = value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into map expects tuple {{key, value}}, found {}",
                    value.kind_label()
                )));
            };

            let key = *key;
            let entry_value = *entry_value;
            if let Some(existing) = entries.iter_mut().find(|(entry_key, _)| *entry_key == key) {
                existing.1 = entry_value;
            } else {
                entries.push((key, entry_value));
            }
        }
        StaticForCollector::Keyword(entries) => {
            let StaticForValue::Tuple(key, entry_value) = value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into keyword expects tuple {{key, value}}, found {}",
                    value.kind_label()
                )));
            };

            let key = *key;
            if !matches!(key, StaticForValue::Atom(_)) {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into keyword expects atom key, found {}",
                    key.kind_label()
                )));
            }

            entries.push((key, *entry_value));
        }
        StaticForCollector::Reduce(_) => {
            return Err(StaticForEvalIssue::Runtime(
                "for internal error: reduce collector cannot accept yielded values".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrForGenerator, IrOp, IrPattern};

    fn evaluate_generator_source(source_ops: Vec<IrOp>) -> Vec<StaticForValue> {
        let for_op = IrOp::For {
            generators: vec![IrForGenerator {
                pattern: IrPattern::Bind {
                    name: "x".to_string(),
                },
                source_ops,
                guard_ops: None,
            }],
            into_ops: None,
            reduce_ops: None,
            body_ops: vec![
                IrOp::LoadVariable {
                    name: "x".to_string(),
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
            offset: 0,
        };

        let StaticForValue::List(values) =
            evaluate_for_spec(&for_op).expect("static for should evaluate")
        else {
            panic!("static for should collect to list");
        };

        values
    }

    #[test]
    fn static_for_iterates_range_generator() {
        let values = evaluate_generator_source(vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 3,
                offset: 0,
            },
            IrOp::Range { offset: 0 },
        ]);

        assert_eq!(
            values,
            vec![
                StaticForValue::Int(1),
                StaticForValue::Int(2),
                StaticForValue::Int(3),
            ]
        );
    }

    #[test]
    fn static_for_iterates_stepped_range_generator() {
        let values = evaluate_generator_source(vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 10,
                offset: 0,
            },
            IrOp::Range { offset: 0 },
            IrOp::ConstInt {
                value: 3,
                offset: 0,
            },
            IrOp::SteppedRange { offset: 0 },
        ]);

        assert_eq!(
            values,
            vec![
                StaticForValue::Int(1),
                StaticForValue::Int(4),
                StaticForValue::Int(7),
                StaticForValue::Int(10),
            ]
        );
    }

    #[test]
    fn static_for_iterates_descending_stepped_range_generator() {
        let values = evaluate_generator_source(vec![
            IrOp::ConstInt {
                value: 10,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::Range { offset: 0 },
            IrOp::ConstInt {
                value: -3,
                offset: 0,
            },
            IrOp::SteppedRange { offset: 0 },
        ]);

        assert_eq!(
            values,
            vec![
                StaticForValue::Int(10),
                StaticForValue::Int(7),
                StaticForValue::Int(4),
                StaticForValue::Int(1),
            ]
        );
    }

    #[test]
    fn static_for_zero_step_stepped_range_is_empty() {
        let values = evaluate_generator_source(vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::Range { offset: 0 },
            IrOp::ConstInt {
                value: 0,
                offset: 0,
            },
            IrOp::SteppedRange { offset: 0 },
        ]);

        assert!(values.is_empty());
    }
}
