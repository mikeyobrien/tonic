use std::collections::BTreeMap;

use crate::ir::{IrForGenerator, IrOp};
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::hash_ir_op_i64;
use super::stubs::c_string_literal;
use crate::cli_diag::failure_message_lines_with_filename_and_source;

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
            out.push_str("  tn_binding_restore(tn_for_bindings, tn_for_bindings_len);\n");
            out.push_str("  return tn_stub_abort(\"tn_runtime_for\");\n");
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
    let values = match enumerable {
        StaticForValue::List(values) => values,
        other => {
            return Err(StaticForEvalIssue::Runtime(format!(
                "for expects list generator, found {}",
                other.kind_label()
            )));
        }
    };

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
