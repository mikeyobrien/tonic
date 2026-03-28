use crate::ir::{CmpKind, IrCallTarget, IrCaseBranch, IrForGenerator, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;

use super::super::error::CBackendError;
use super::super::hash::{hash_pattern_i64, hash_text_i64};
use super::super::stubs::{c_string_literal, pop_stack_value};
use super::{ForSpec, FOR_REDUCE_ACC_BINDING};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForCollectorKind {
    List,
    Map,
    Keyword,
    Reduce,
}

pub(super) fn emit_dynamic_for_case(
    index: usize,
    for_spec: &ForSpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    let IrOp::For {
        generators,
        into_ops,
        reduce_ops,
        body_ops,
        ..
    } = &for_spec.op
    else {
        return Err(CBackendError::new(
            "c backend internal error: dynamic for case source was not IrOp::For",
        ));
    };

    let collector_kind = if reduce_ops.is_some() {
        ForCollectorKind::Reduce
    } else if let Some(into_ops) = into_ops {
        match detect_literal_collection_kind(into_ops) {
            Some(
                kind @ (ForCollectorKind::List | ForCollectorKind::Map | ForCollectorKind::Keyword),
            ) => kind,
            _ => ForCollectorKind::List,
        }
    } else {
        ForCollectorKind::List
    };

    out.push_str(&format!(
        "static TnVal tn_runtime_for_case_{index}(void) {{\n"
    ));
    out.push_str("  TnBinding tn_for_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t tn_for_bindings_len = 0;\n");
    out.push_str("  tn_binding_snapshot(tn_for_bindings, &tn_for_bindings_len);\n");
    out.push_str("  int tn_for_failed = 0;\n");
    out.push_str("  TnVal tn_for_error = tn_runtime_const_nil();\n");
    out.push_str("  TnVal tn_for_result = tn_runtime_const_nil();\n");

    match reduce_ops {
        Some(reduce_ops) => {
            emit_dynamic_for_ops(
                reduce_ops,
                "tn_for_result",
                "tn_for_failed",
                "tn_for_error",
                &format!("tn_for_case_{index}_reduce_init"),
                "  ",
                out,
            )?;
        }
        None => {
            if let Some(into_ops) = into_ops {
                emit_dynamic_for_ops(
                    into_ops,
                    "tn_for_result",
                    "tn_for_failed",
                    "tn_for_error",
                    &format!("tn_for_case_{index}_into_init"),
                    "  ",
                    out,
                )?;
                let expected_check = match collector_kind {
                    ForCollectorKind::List => "tn_runtime_is_list(tn_for_result)",
                    ForCollectorKind::Map => "tn_runtime_is_map(tn_for_result)",
                    ForCollectorKind::Keyword => "tn_runtime_is_keyword(tn_for_result)",
                    ForCollectorKind::Reduce => unreachable!(),
                };
                out.push_str("  if (tn_for_failed == 0) {\n");
                out.push_str(&format!("    if (!({expected_check})) {{\n"));
                out.push_str("      tn_for_failed = 1;\n");
                out.push_str("      tn_for_error = tn_runtime_failf(\"for into destination must be a list, map, or keyword, found %s\", tn_runtime_value_kind(tn_for_result));\n");
                out.push_str("    }\n");
                out.push_str("  }\n");
            } else {
                out.push_str("  tn_for_result = tn_runtime_make_list_varargs((TnVal)0);\n");
            }
        }
    }

    emit_dynamic_for_generators(
        generators,
        0,
        body_ops,
        collector_kind,
        "tn_for_result",
        "tn_for_failed",
        "tn_for_error",
        &format!("tn_for_case_{index}"),
        "  ",
        out,
    )?;

    out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
    out.push_str("  if (tn_for_failed != 0) {\n");
    out.push_str("    return tn_for_error;\n");
    out.push_str("  }\n");
    out.push_str("  return tn_for_result;\n");
    out.push_str("}\n\n");
    Ok(())
}

fn detect_literal_collection_kind(ops: &[IrOp]) -> Option<ForCollectorKind> {
    match ops {
        [IrOp::Call {
            callee: IrCallTarget::Builtin { name },
            argc,
            ..
        }] if name == "map_empty" && *argc == 0 => Some(ForCollectorKind::Map),
        [IrOp::Call {
            callee: IrCallTarget::Builtin { name },
            argc,
            ..
        }] if name == "list" && *argc == 0 => Some(ForCollectorKind::List),
        [IrOp::Call {
            callee: IrCallTarget::Builtin { name },
            argc,
            ..
        }] if name == "keyword" && *argc == 2 => Some(ForCollectorKind::Keyword),
        _ => None,
    }
}

fn emit_dynamic_for_generators(
    generators: &[IrForGenerator],
    index: usize,
    body_ops: &[IrOp],
    collector_kind: ForCollectorKind,
    result_var: &str,
    failed_var: &str,
    error_var: &str,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    if index >= generators.len() {
        emit_dynamic_for_collect(
            body_ops,
            collector_kind,
            result_var,
            failed_var,
            error_var,
            &format!("{label}_body"),
            indent,
            out,
        )?;
        return Ok(());
    }

    let generator = &generators[index];
    let source_var = format!("{label}_source_{index}");
    out.push_str(&format!(
        "{indent}TnVal {source_var} = tn_runtime_const_nil();\n"
    ));
    emit_dynamic_for_ops(
        &generator.source_ops,
        &source_var,
        failed_var,
        error_var,
        &format!("{label}_gen_{index}_source"),
        indent,
        out,
    )?;
    out.push_str(&format!("{indent}if ({failed_var} == 0) {{\n"));

    let pattern_hash = hash_pattern_i64(&generator.pattern)?;
    let iter_snapshot = format!("{label}_gen_{index}_bindings");
    let iter_snapshot_len = format!("{label}_gen_{index}_bindings_len");
    let list_obj = format!("{label}_gen_{index}_list_obj");
    let map_obj = format!("{label}_gen_{index}_map_obj");
    let range_obj = format!("{label}_gen_{index}_range_obj");

    out.push_str(&format!(
        "{indent}TnObj *{list_obj} = tn_get_obj({source_var});\n"
    ));
    out.push_str(&format!(
        "{indent}if ({list_obj} != NULL && {list_obj}->kind == TN_OBJ_LIST) {{\n"
    ));
    out.push_str(&format!(
        "{indent}  for (size_t {label}_gen_{index}_item = 0; {label}_gen_{index}_item < {list_obj}->as.list.len; {label}_gen_{index}_item += 1) {{\n"
    ));
    emit_dynamic_for_iteration(
        index,
        &format!("{list_obj}->as.list.items[{label}_gen_{index}_item]"),
        pattern_hash,
        generator.guard_ops.as_deref(),
        generators,
        body_ops,
        collector_kind,
        result_var,
        failed_var,
        error_var,
        label,
        &format!("{indent}    "),
        &iter_snapshot,
        &iter_snapshot_len,
        out,
    )?;
    out.push_str(&format!("{indent}  }}\n"));
    out.push_str(&format!("{indent}}} else {{\n"));
    out.push_str(&format!(
        "{indent}  TnObj *{range_obj} = tn_get_obj({source_var});\n"
    ));
    out.push_str(&format!(
        "{indent}  if ({range_obj} != NULL && {range_obj}->kind == TN_OBJ_RANGE) {{\n"
    ));
    out.push_str(&format!(
        "{indent}    TnVal {label}_gen_{index}_start = {range_obj}->as.range.start;\n"
    ));
    out.push_str(&format!(
        "{indent}    TnVal {label}_gen_{index}_end = {range_obj}->as.range.end;\n"
    ));
    out.push_str(&format!(
        "{indent}    if ({label}_gen_{index}_start <= {label}_gen_{index}_end) {{\n"
    ));
    out.push_str(&format!(
        "{indent}      for (TnVal {label}_gen_{index}_item = {label}_gen_{index}_start; {label}_gen_{index}_item <= {label}_gen_{index}_end; {label}_gen_{index}_item += 1) {{\n"
    ));
    emit_dynamic_for_iteration(
        index,
        &format!("{label}_gen_{index}_item"),
        pattern_hash,
        generator.guard_ops.as_deref(),
        generators,
        body_ops,
        collector_kind,
        result_var,
        failed_var,
        error_var,
        label,
        &format!("{indent}        "),
        &iter_snapshot,
        &iter_snapshot_len,
        out,
    )?;
    out.push_str(&format!("{indent}      }}\n"));
    out.push_str(&format!("{indent}    }}\n"));
    out.push_str(&format!("{indent}  }} else {{\n"));
    out.push_str(&format!(
        "{indent}    TnObj *{map_obj} = tn_get_obj({source_var});\n"
    ));
    out.push_str(&format!(
        "{indent}    if ({map_obj} != NULL && ({map_obj}->kind == TN_OBJ_MAP || {map_obj}->kind == TN_OBJ_KEYWORD)) {{\n"
    ));
    out.push_str(&format!(
        "{indent}      for (size_t {label}_gen_{index}_pair = 0; {label}_gen_{index}_pair < {map_obj}->as.map_like.len; {label}_gen_{index}_pair += 1) {{\n"
    ));
    out.push_str(&format!("{indent}        TnVal {label}_gen_{index}_pair_value = tn_runtime_make_tuple({map_obj}->as.map_like.items[{label}_gen_{index}_pair].key, {map_obj}->as.map_like.items[{label}_gen_{index}_pair].value);\n"));
    emit_dynamic_for_iteration(
        index,
        &format!("{label}_gen_{index}_pair_value"),
        pattern_hash,
        generator.guard_ops.as_deref(),
        generators,
        body_ops,
        collector_kind,
        result_var,
        failed_var,
        error_var,
        label,
        &format!("{indent}        "),
        &iter_snapshot,
        &iter_snapshot_len,
        out,
    )?;
    out.push_str(&format!("{indent}      }}\n"));
    out.push_str(&format!("{indent}    }} else {{\n"));
    out.push_str(&format!("{indent}      {failed_var} = 1;\n"));
    out.push_str(&format!("{indent}      {error_var} = tn_runtime_failf(\"for requires iterable, found %s\", tn_runtime_value_kind({source_var}));\n"));
    out.push_str(&format!("{indent}    }}\n"));
    out.push_str(&format!("{indent}  }}\n"));
    out.push_str(&format!("{indent}}}\n"));
    out.push_str(&format!("{indent}}}\n"));

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_dynamic_for_iteration(
    index: usize,
    item_expr: &str,
    pattern_hash: i64,
    guard_ops: Option<&[IrOp]>,
    generators: &[IrForGenerator],
    body_ops: &[IrOp],
    collector_kind: ForCollectorKind,
    result_var: &str,
    failed_var: &str,
    error_var: &str,
    label: &str,
    indent: &str,
    iter_snapshot: &str,
    iter_snapshot_len: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "{indent}TnBinding {iter_snapshot}[TN_MAX_BINDINGS];\n"
    ));
    out.push_str(&format!("{indent}size_t {iter_snapshot_len} = 0;\n"));
    out.push_str(&format!(
        "{indent}tn_binding_snapshot({iter_snapshot}, &{iter_snapshot_len});\n"
    ));
    out.push_str(&format!(
        "{indent}if (!tn_runtime_pattern_matches({item_expr}, (TnVal){pattern_hash}LL)) {{\n"
    ));
    out.push_str(&format!(
        "{indent}  tn_binding_restore({iter_snapshot}, {iter_snapshot_len});\n"
    ));
    out.push_str(&format!("{indent}  continue;\n"));
    out.push_str(&format!("{indent}}}\n"));

    if let Some(guard_ops) = guard_ops {
        let guard_var = format!("{label}_gen_{index}_guard_value");
        out.push_str(&format!(
            "{indent}TnVal {guard_var} = tn_runtime_const_nil();\n"
        ));
        emit_dynamic_for_ops(
            guard_ops,
            &guard_var,
            failed_var,
            error_var,
            &format!("{label}_gen_{index}_guard"),
            indent,
            out,
        )?;
        out.push_str(&format!("{indent}if ({failed_var} != 0) {{\n"));
        out.push_str(&format!(
            "{indent}  tn_binding_restore({iter_snapshot}, {iter_snapshot_len});\n"
        ));
        out.push_str(&format!("{indent}  break;\n"));
        out.push_str(&format!("{indent}}}\n"));
        out.push_str(&format!(
            "{indent}if (!tn_runtime_is_truthy({guard_var})) {{\n"
        ));
        out.push_str(&format!(
            "{indent}  tn_binding_restore({iter_snapshot}, {iter_snapshot_len});\n"
        ));
        out.push_str(&format!("{indent}  continue;\n"));
        out.push_str(&format!("{indent}}}\n"));
    }

    emit_dynamic_for_generators(
        generators,
        index + 1,
        body_ops,
        collector_kind,
        result_var,
        failed_var,
        error_var,
        label,
        indent,
        out,
    )?;
    out.push_str(&format!(
        "{indent}tn_binding_restore({iter_snapshot}, {iter_snapshot_len});\n"
    ));
    out.push_str(&format!("{indent}if ({failed_var} != 0) {{\n"));
    out.push_str(&format!("{indent}  break;\n"));
    out.push_str(&format!("{indent}}}\n"));
    Ok(())
}

fn emit_dynamic_for_collect(
    body_ops: &[IrOp],
    collector_kind: ForCollectorKind,
    result_var: &str,
    failed_var: &str,
    error_var: &str,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    let body_var = format!("{label}_value");
    out.push_str(&format!(
        "{indent}TnVal {body_var} = tn_runtime_const_nil();\n"
    ));

    if collector_kind == ForCollectorKind::Reduce {
        let acc_hash = hash_text_i64(FOR_REDUCE_ACC_BINDING);
        out.push_str(&format!(
            "{indent}tn_binding_set((TnVal){acc_hash}LL, {result_var});\n"
        ));
    }

    emit_dynamic_for_ops(
        body_ops, &body_var, failed_var, error_var, label, indent, out,
    )?;
    out.push_str(&format!("{indent}if ({failed_var} == 0) {{\n"));

    match collector_kind {
        ForCollectorKind::List => {
            let single = format!("{label}_single");
            out.push_str(&format!(
                "{indent}TnVal {single} = tn_runtime_make_list_varargs((TnVal)1, {body_var});\n"
            ));
            out.push_str(&format!(
                "{indent}{result_var} = tn_runtime_list_concat({result_var}, {single});\n"
            ));
        }
        ForCollectorKind::Map => {
            let pair_left = format!("{label}_pair_left");
            let pair_right = format!("{label}_pair_right");
            out.push_str(&format!(
                "{indent}TnObj *{label}_pair_obj = tn_get_obj({body_var});\n"
            ));
            out.push_str(&format!("{indent}if ({label}_pair_obj == NULL || {label}_pair_obj->kind != TN_OBJ_TUPLE) {{\n"));
            out.push_str(&format!("{indent}  {failed_var} = 1;\n"));
            out.push_str(&format!("{indent}  {error_var} = tn_runtime_failf(\"for into map expects tuple {{key, value}}, found %s\", tn_runtime_value_kind({body_var}));\n"));
            out.push_str(&format!("{indent}}} else {{\n"));
            out.push_str(&format!(
                "{indent}  TnVal {pair_left} = {label}_pair_obj->as.tuple.left;\n"
            ));
            out.push_str(&format!(
                "{indent}  TnVal {pair_right} = {label}_pair_obj->as.tuple.right;\n"
            ));
            out.push_str(&format!(
                "{indent}  {result_var} = tn_runtime_map_put({result_var}, {pair_left}, {pair_right});\n"
            ));
            out.push_str(&format!("{indent}}}\n"));
        }
        ForCollectorKind::Keyword => {
            let pair_left = format!("{label}_pair_left");
            let pair_right = format!("{label}_pair_right");
            out.push_str(&format!(
                "{indent}TnObj *{label}_pair_obj = tn_get_obj({body_var});\n"
            ));
            out.push_str(&format!("{indent}if ({label}_pair_obj == NULL || {label}_pair_obj->kind != TN_OBJ_TUPLE) {{\n"));
            out.push_str(&format!("{indent}  {failed_var} = 1;\n"));
            out.push_str(&format!("{indent}  {error_var} = tn_runtime_failf(\"for into keyword expects tuple {{key, value}}, found %s\", tn_runtime_value_kind({body_var}));\n"));
            out.push_str(&format!("{indent}}} else {{\n"));
            out.push_str(&format!(
                "{indent}  TnVal {pair_left} = {label}_pair_obj->as.tuple.left;\n"
            ));
            out.push_str(&format!(
                "{indent}  TnVal {pair_right} = {label}_pair_obj->as.tuple.right;\n"
            ));
            out.push_str(&format!(
                "{indent}  if (!tn_runtime_is_atom({pair_left})) {{\n"
            ));
            out.push_str(&format!("{indent}    {failed_var} = 1;\n"));
            out.push_str(&format!("{indent}    {error_var} = tn_runtime_failf(\"for into keyword expects atom key, found %s\", tn_runtime_value_kind({pair_left}));\n"));
            out.push_str(&format!("{indent}  }} else {{\n"));
            out.push_str(&format!(
                "{indent}    {result_var} = tn_runtime_keyword_append({result_var}, {pair_left}, {pair_right});\n"
            ));
            out.push_str(&format!("{indent}  }}\n"));
            out.push_str(&format!("{indent}}}\n"));
        }
        ForCollectorKind::Reduce => {
            out.push_str(&format!("{indent}{result_var} = {body_var};\n"));
        }
    }

    out.push_str(&format!("{indent}}}\n"));

    Ok(())
}

pub(super) fn emit_dynamic_for_ops(
    ops: &[IrOp],
    result_var: &str,
    failed_var: &str,
    error_var: &str,
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
            IrOp::AddInt { .. } => {
                emit_dynamic_binary("+", &mut stack, &mut temp_index, label, indent, out)?
            }
            IrOp::SubInt { .. } => {
                emit_dynamic_binary("-", &mut stack, &mut temp_index, label, indent, out)?
            }
            IrOp::MulInt { .. } => {
                emit_dynamic_binary("*", &mut stack, &mut temp_index, label, indent, out)?
            }
            IrOp::DivInt { .. } | IrOp::IntDiv { .. } => {
                emit_dynamic_binary("/", &mut stack, &mut temp_index, label, indent, out)?
            }
            IrOp::RemInt { .. } => {
                emit_dynamic_binary("%", &mut stack, &mut temp_index, label, indent, out)?
            }
            IrOp::Range { .. } => {
                let right = pop_stack_value(&mut stack, "for range right operand")?;
                let left = pop_stack_value(&mut stack, "for range left operand")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_range({left}, {right});\n"
                ));
                stack.push(temp);
            }
            IrOp::SteppedRange { .. } => {
                let step = pop_stack_value(&mut stack, "for stepped range step")?;
                let range = pop_stack_value(&mut stack, "for stepped range range")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_stepped_range({range}, {step});\n"
                ));
                stack.push(temp);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_stack_value(&mut stack, "for cmp right operand")?;
                let left = pop_stack_value(&mut stack, "for cmp left operand")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                match kind {
                    CmpKind::Eq | CmpKind::StrictEq => {
                        out.push_str(&format!(
                            "{indent}  TnVal {temp} = tn_runtime_const_bool(tn_runtime_value_equal({left}, {right}) ? 1 : 0);\n"
                        ));
                    }
                    CmpKind::NotEq | CmpKind::StrictNotEq => {
                        out.push_str(&format!(
                            "{indent}  TnVal {temp} = tn_runtime_const_bool(tn_runtime_value_equal({left}, {right}) ? 0 : 1);\n"
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
                            "{indent}  TnVal {temp} = tn_runtime_const_bool(({left} {operator} {right}) ? 1 : 0);\n"
                        ));
                    }
                }
                stack.push(temp);
            }
            IrOp::ToString { .. } => {
                let input = pop_stack_value(&mut stack, "for to_string input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_to_string({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Not { .. } => {
                let input = pop_stack_value(&mut stack, "for not input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_not({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Bang { .. } => {
                let input = pop_stack_value(&mut stack, "for bang input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_bang({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Question { .. } => {
                let input = pop_stack_value(&mut stack, "for question input")?;
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_question({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                emit_dynamic_call(
                    callee,
                    *argc,
                    &mut stack,
                    &mut temp_index,
                    label,
                    indent,
                    out,
                )?;
            }
            IrOp::CallValue { argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(&mut stack, "for call_value argument")?);
                }
                args.reverse();
                let callee = pop_stack_value(&mut stack, "for call_value callee")?;
                let root_frame = format!("{label}_rf_{temp_index}");
                out.push_str(&format!(
                    "{indent}  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                out.push_str(&format!("{indent}  tn_runtime_root_register({callee});\n"));
                for argument in &args {
                    out.push_str(&format!(
                        "{indent}  tn_runtime_root_register({argument});\n"
                    ));
                }
                let call_args = std::iter::once(callee)
                    .chain(std::iter::once(format!("(TnVal){argc}")))
                    .chain(args.into_iter())
                    .collect::<Vec<_>>()
                    .join(", ");
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_call_closure_varargs({call_args});\n"
                ));
                out.push_str(&format!("{indent}  tn_runtime_retain({temp});\n"));
                out.push_str(&format!(
                    "{indent}  tn_runtime_root_frame_pop({root_frame});\n"
                ));
                out.push_str(&format!("{indent}  tn_runtime_root_register({temp});\n"));
                out.push_str(&format!("{indent}  tn_runtime_release({temp});\n"));
                stack.push(temp);
            }
            IrOp::Case { branches, .. } => {
                emit_dynamic_case(
                    branches,
                    &mut stack,
                    &mut temp_index,
                    result_var,
                    failed_var,
                    error_var,
                    label,
                    indent,
                    out,
                )?;
            }
            IrOp::Drop => {
                stack.pop();
            }
            IrOp::Return { .. } => {
                let value = pop_stack_value(&mut stack, "for return value")?;
                out.push_str(&format!("{indent}  {result_var} = {value};\n"));
                out.push_str(&format!("{indent}  break;\n"));
                terminated = true;
                break;
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend for helper unsupported op: {other:?}"
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

#[allow(clippy::too_many_arguments)]
fn emit_dynamic_case(
    branches: &[IrCaseBranch],
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    result_var: &str,
    failed_var: &str,
    error_var: &str,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    let subject = pop_stack_value(stack, "for case subject")?;
    let case_index = *temp_index;
    *temp_index += 1;
    let case_result = format!("{label}_case_{case_index}");
    let case_matched = format!("{label}_case_matched_{case_index}");
    let case_bindings = format!("{label}_case_bindings_{case_index}");
    let case_bindings_len = format!("{label}_case_bindings_len_{case_index}");

    out.push_str(&format!(
        "{indent}  TnVal {case_result} = tn_runtime_const_nil();\n"
    ));
    out.push_str(&format!("{indent}  int {case_matched} = 0;\n"));
    out.push_str(&format!(
        "{indent}  TnBinding {case_bindings}[TN_MAX_BINDINGS];\n"
    ));
    out.push_str(&format!("{indent}  size_t {case_bindings_len} = 0;\n"));
    out.push_str(&format!(
        "{indent}  tn_binding_snapshot({case_bindings}, &{case_bindings_len});\n"
    ));

    for (branch_index, branch) in branches.iter().enumerate() {
        let condition = if matches!(branch.pattern, IrPattern::Wildcard) {
            "1".to_string()
        } else {
            let pattern_hash = hash_pattern_i64(&branch.pattern)?;
            format!("tn_runtime_pattern_matches({subject}, (TnVal){pattern_hash}LL)")
        };

        out.push_str(&format!(
            "{indent}  if (!{case_matched}) {{\n{indent}    tn_binding_restore({case_bindings}, {case_bindings_len});\n{indent}    if ({condition}) {{\n"
        ));

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_var = format!("{label}_case_guard_{case_index}_{branch_index}");
            out.push_str(&format!(
                "{indent}      TnVal {guard_var} = tn_runtime_const_nil();\n"
            ));
            emit_dynamic_for_ops(
                guard_ops,
                &guard_var,
                failed_var,
                error_var,
                &format!("{label}_case_{case_index}_guard_{branch_index}"),
                &format!("{indent}      "),
                out,
            )?;
            out.push_str(&format!("{indent}      if ({failed_var} != 0) {{\n"));
            out.push_str(&format!("{indent}        break;\n"));
            out.push_str(&format!("{indent}      }}\n"));
            out.push_str(&format!(
                "{indent}      if (tn_runtime_is_truthy({guard_var})) {{\n"
            ));
        }

        let branch_indent = if branch.guard_ops.is_some() {
            format!("{indent}        ")
        } else {
            format!("{indent}      ")
        };
        emit_dynamic_for_ops(
            &branch.ops,
            &case_result,
            failed_var,
            error_var,
            &format!("{label}_case_{case_index}_branch_{branch_index}"),
            &branch_indent,
            out,
        )?;
        out.push_str(&format!("{branch_indent}if ({failed_var} == 0) {{\n"));
        out.push_str(&format!("{branch_indent}  {case_matched} = 1;\n"));
        out.push_str(&format!("{branch_indent}}}\n"));

        if branch.guard_ops.is_some() {
            out.push_str(&format!("{indent}      }}\n"));
        }

        out.push_str(&format!("{indent}    }}\n{indent}  }}\n"));
    }

    out.push_str(&format!(
        "{indent}  tn_binding_restore({case_bindings}, {case_bindings_len});\n"
    ));
    out.push_str(&format!(
        "{indent}  if ({failed_var} == 0 && !{case_matched}) {{\n"
    ));
    out.push_str(&format!("{indent}    {failed_var} = 1;\n"));
    out.push_str(&format!(
        "{indent}    {error_var} = tn_runtime_fail(\"no case clause matching\");\n"
    ));
    out.push_str(&format!("{indent}    break;\n"));
    out.push_str(&format!("{indent}  }}\n"));

    stack.push(case_result);
    let _ = result_var;
    Ok(())
}

fn emit_dynamic_binary(
    operator: &str,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    let right = pop_stack_value(stack, "for binary right operand")?;
    let left = pop_stack_value(stack, "for binary left operand")?;
    let temp = format!("{label}_tmp_{}", *temp_index);
    *temp_index += 1;
    out.push_str(&format!(
        "{indent}  TnVal {temp} = {left} {operator} {right};\n"
    ));
    stack.push(temp);
    Ok(())
}

fn emit_dynamic_call(
    callee: &IrCallTarget,
    argc: usize,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_stack_value(stack, "for call argument")?);
    }
    args.reverse();

    let rendered_args = args.join(", ");
    let temp = format!("{label}_tmp_{}", *temp_index);
    let root_frame = format!("{label}_rf_{}", *temp_index);
    *temp_index += 1;

    out.push_str(&format!(
        "{indent}  size_t {root_frame} = tn_runtime_root_frame_push();\n"
    ));
    for argument in &args {
        out.push_str(&format!(
            "{indent}  tn_runtime_root_register({argument});\n"
        ));
    }

    match callee {
        IrCallTarget::Builtin { name } => {
            emit_dynamic_builtin_call(&temp, name, &args, &rendered_args, argc, indent, out)?;
        }
        IrCallTarget::Function { name } => {
            let symbol = mangle_function_name(name, argc);
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
    Ok(())
}

fn emit_dynamic_builtin_call(
    temp: &str,
    builtin: &str,
    args: &[String],
    rendered_args: &str,
    argc: usize,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    match builtin {
        "ok" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_make_ok({rendered_args});\n"
        )),
        "err" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_make_err({rendered_args});\n"
        )),
        "tuple" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_make_tuple({rendered_args});\n"
        )),
        "list" => {
            let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "{indent}  TnVal {temp} = tn_runtime_make_list_varargs({count_then_args});\n"
            ));
        }
        "bitstring" => {
            let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "{indent}  TnVal {temp} = tn_runtime_make_bitstring_varargs({count_then_args});\n"
            ));
        }
        "map_empty" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_map_empty();\n"
        )),
        "map" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_make_map({rendered_args});\n"
        )),
        "map_put" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_map_put({rendered_args});\n"
        )),
        "map_update" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_map_update({rendered_args});\n"
        )),
        "map_access" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_map_access({rendered_args});\n"
        )),
        "keyword" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_make_keyword({rendered_args});\n"
        )),
        "keyword_append" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_keyword_append({rendered_args});\n"
        )),
        "host_call" => {
            let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "{indent}  TnVal {temp} = tn_runtime_host_call_varargs({count_then_args});\n"
            ));
        }
        "protocol_dispatch" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_protocol_dispatch({rendered_args});\n"
        )),
        "div" => out.push_str(&format!(
            "{indent}  TnVal {temp} = (TnVal)({} / {});\n",
            args[0], args[1]
        )),
        "rem" => out.push_str(&format!(
            "{indent}  TnVal {temp} = (TnVal)({} % {});\n",
            args[0], args[1]
        )),
        "byte_size" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_byte_size({rendered_args});\n"
        )),
        "bit_size" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_bit_size({rendered_args});\n"
        )),
        "abs" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_abs({rendered_args});\n"
        )),
        "length" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_length({rendered_args});\n"
        )),
        "hd" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_hd({rendered_args});\n"
        )),
        "tl" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_tl({rendered_args});\n"
        )),
        "elem" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_elem({rendered_args});\n"
        )),
        "tuple_size" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_tuple_size({rendered_args});\n"
        )),
        "to_string" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_to_string({rendered_args});\n"
        )),
        "max" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_max({rendered_args});\n"
        )),
        "min" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_min({rendered_args});\n"
        )),
        "round" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_round({rendered_args});\n"
        )),
        "trunc" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_trunc({rendered_args});\n"
        )),
        "map_size" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_map_size({rendered_args});\n"
        )),
        "put_elem" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_put_elem({rendered_args});\n"
        )),
        "inspect" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_inspect({rendered_args});\n"
        )),
        "is_integer" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_const_bool(tn_runtime_is_integer({rendered_args}) ? 1 : 0);\n"
        )),
        "is_number" => out.push_str(&format!(
            "{indent}  TnVal {temp} = tn_runtime_const_bool(tn_runtime_is_number({rendered_args}) ? 1 : 0);\n"
        )),
        other => {
            return Err(CBackendError::new(format!(
                "c backend for helper unsupported builtin call target: {other}"
            )))
        }
    }
    Ok(())
}
