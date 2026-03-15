use std::collections::BTreeMap;

use crate::ir::{CmpKind, IrCallTarget, IrForGenerator, IrOp, IrPattern};
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::hash_ir_op_i64;
use super::stubs::c_string_literal;

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
    out: &mut String,
) -> Result<(), CBackendError> {
    let for_specs = collect_for_specs(mir)?;

    out.push_str("/* compiled for helpers */\n");
    for (index, for_spec) in for_specs.iter().enumerate() {
        emit_runtime_for_case(index, for_spec, out)?;
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
            let escaped = c_string_literal(&message);
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

fn evaluate_static_for_ops(
    ops: &[IrOp],
    env: &BTreeMap<String, StaticForValue>,
) -> Result<StaticForValue, StaticForEvalIssue> {
    let mut stack = Vec::<StaticForValue>::new();

    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => stack.push(StaticForValue::Int(*value)),
            IrOp::ConstBool { value, .. } => stack.push(StaticForValue::Bool(*value)),
            IrOp::ConstNil { .. } => stack.push(StaticForValue::Nil),
            IrOp::ConstAtom { value, .. } => stack.push(StaticForValue::Atom(value.clone())),
            IrOp::ConstString { value, .. } => stack.push(StaticForValue::String(value.clone())),
            IrOp::ConstFloat { value, .. } => stack.push(StaticForValue::Float(value.clone())),
            IrOp::LoadVariable { name, .. } => {
                if let Some(value) = env.get(name) {
                    stack.push(value.clone());
                } else {
                    return Err(StaticForEvalIssue::Unsupported(format!(
                        "for helper unknown binding '{name}'"
                    )));
                }
            }
            IrOp::AddInt { .. } => {
                let right = pop_static_for_int(&mut stack, "add right")?;
                let left = pop_static_for_int(&mut stack, "add left")?;
                stack.push(StaticForValue::Int(left + right));
            }
            IrOp::SubInt { .. } => {
                let right = pop_static_for_int(&mut stack, "sub right")?;
                let left = pop_static_for_int(&mut stack, "sub left")?;
                stack.push(StaticForValue::Int(left - right));
            }
            IrOp::MulInt { .. } => {
                let right = pop_static_for_int(&mut stack, "mul right")?;
                let left = pop_static_for_int(&mut stack, "mul left")?;
                stack.push(StaticForValue::Int(left * right));
            }
            IrOp::DivInt { .. } => {
                let right = pop_static_for_int(&mut stack, "div right")?;
                let left = pop_static_for_int(&mut stack, "div left")?;
                stack.push(StaticForValue::Int(left / right));
            }
            IrOp::Case { branches, .. } => {
                let subject = pop_static_for_value(&mut stack, "case subject")?;
                let mut matched_value = None;

                for branch in branches {
                    let mut branch_env = env.clone();
                    if !apply_pattern_bindings(&branch.pattern, &subject, &mut branch_env)? {
                        continue;
                    }

                    if let Some(guard_ops) = &branch.guard_ops {
                        let guard_value = evaluate_static_for_ops(guard_ops, &branch_env)?;
                        let StaticForValue::Bool(guard_result) = guard_value else {
                            return Err(StaticForEvalIssue::Runtime(format!(
                                "for helper case guard must evaluate to bool, found {}",
                                guard_value.kind_label()
                            )));
                        };

                        if !guard_result {
                            continue;
                        }
                    }

                    matched_value = Some(evaluate_static_for_ops(&branch.ops, &branch_env)?);
                    break;
                }

                if let Some(value) = matched_value {
                    stack.push(value);
                } else {
                    return Err(StaticForEvalIssue::Runtime(
                        "no case clause matching".to_string(),
                    ));
                }
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_static_for_int(&mut stack, "cmp right")?;
                let left = pop_static_for_int(&mut stack, "cmp left")?;
                let result = match kind {
                    CmpKind::Eq => left == right,
                    CmpKind::NotEq => left != right,
                    CmpKind::Lt => left < right,
                    CmpKind::Lte => left <= right,
                    CmpKind::Gt => left > right,
                    CmpKind::Gte => left >= right,
                    CmpKind::StrictEq | CmpKind::StrictNotEq => {
                        return Err(StaticForEvalIssue::Unsupported(
                            "strict equality not supported in static for".to_string(),
                        ))
                    }
                };
                stack.push(StaticForValue::Bool(result));
            }
            IrOp::Call { callee, argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_static_for_value(&mut stack, "call argument")?);
                }
                args.reverse();

                match callee {
                    IrCallTarget::Builtin { name } => match name.as_str() {
                        "tuple" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper tuple expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Tuple(
                                Box::new(args[0].clone()),
                                Box::new(args[1].clone()),
                            ));
                        }
                        "list" => stack.push(StaticForValue::List(args)),
                        "map_empty" => {
                            if !args.is_empty() {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map_empty expected 0 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Map(Vec::new()));
                        }
                        "map" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Map(vec![(
                                args[0].clone(),
                                args[1].clone(),
                            )]));
                        }
                        "map_put" => {
                            if args.len() != 3 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map_put expected 3 args, found {}",
                                    args.len()
                                )));
                            }

                            let mut entries = match args[0].clone() {
                                StaticForValue::Map(entries) => entries,
                                other => {
                                    return Err(StaticForEvalIssue::Runtime(format!(
                                        "for helper map_put expected map base, found {}",
                                        other.kind_label()
                                    )));
                                }
                            };

                            let key = args[1].clone();
                            let value = args[2].clone();
                            if let Some(existing) =
                                entries.iter_mut().find(|(entry_key, _)| *entry_key == key)
                            {
                                existing.1 = value;
                            } else {
                                entries.push((key, value));
                            }
                            stack.push(StaticForValue::Map(entries));
                        }
                        "keyword" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper keyword expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Keyword(vec![(
                                args[0].clone(),
                                args[1].clone(),
                            )]));
                        }
                        "keyword_append" => {
                            if args.len() != 3 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper keyword_append expected 3 args, found {}",
                                    args.len()
                                )));
                            }

                            let mut entries = match args[0].clone() {
                                StaticForValue::Keyword(entries) => entries,
                                other => {
                                    return Err(StaticForEvalIssue::Runtime(format!(
                                        "for helper keyword_append expected keyword base, found {}",
                                        other.kind_label()
                                    )));
                                }
                            };
                            entries.push((args[1].clone(), args[2].clone()));
                            stack.push(StaticForValue::Keyword(entries));
                        }
                        "is_integer" => {
                            if args.len() != 1 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper is_integer expected 1 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Bool(matches!(
                                args.first(),
                                Some(StaticForValue::Int(_))
                            )));
                        }
                        "is_number" => {
                            if args.len() != 1 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper is_number expected 1 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Bool(matches!(
                                args.first(),
                                Some(StaticForValue::Int(_) | StaticForValue::Float(_))
                            )));
                        }
                        other => {
                            return Err(StaticForEvalIssue::Unsupported(format!(
                                "for helper unsupported builtin call target: {other}"
                            )));
                        }
                    },
                    IrCallTarget::Function { name } => {
                        return Err(StaticForEvalIssue::Unsupported(format!(
                            "for helper unsupported function call target: {name}"
                        )));
                    }
                }
            }
            IrOp::Return { .. } => {
                if let Some(value) = stack.pop() {
                    return Ok(value);
                }
                return Ok(StaticForValue::Nil);
            }
            other => {
                return Err(StaticForEvalIssue::Unsupported(format!(
                    "for helper unsupported op: {other:?}"
                )));
            }
        }
    }

    if let Some(value) = stack.pop() {
        Ok(value)
    } else {
        Ok(StaticForValue::Nil)
    }
}

fn apply_pattern_bindings(
    pattern: &IrPattern,
    value: &StaticForValue,
    env: &mut BTreeMap<String, StaticForValue>,
) -> Result<bool, StaticForEvalIssue> {
    let snapshot = env.clone();

    let matched = match pattern {
        IrPattern::Bind { name } => {
            if let Some(existing) = env.get(name) {
                existing == value
            } else {
                env.insert(name.clone(), value.clone());
                true
            }
        }
        IrPattern::Pin { name } => env.get(name).map(|bound| bound == value).unwrap_or(false),
        IrPattern::Wildcard => true,
        IrPattern::Integer { value: expected } => {
            matches!(value, StaticForValue::Int(actual) if actual == expected)
        }
        IrPattern::Bool { value: expected } => {
            matches!(value, StaticForValue::Bool(actual) if actual == expected)
        }
        IrPattern::Nil => matches!(value, StaticForValue::Nil),
        IrPattern::String { value: expected } => {
            matches!(value, StaticForValue::String(actual) if actual == expected)
        }
        IrPattern::Atom { value: expected } => {
            matches!(value, StaticForValue::Atom(actual) if actual == expected)
        }
        IrPattern::Tuple { items } => {
            if let StaticForValue::Tuple(left, right) = value {
                if items.len() != 2 {
                    false
                } else {
                    apply_pattern_bindings(&items[0], left, env)?
                        && apply_pattern_bindings(&items[1], right, env)?
                }
            } else {
                false
            }
        }
        IrPattern::List { items, tail } => {
            if let StaticForValue::List(values) = value {
                if values.len() < items.len() || (tail.is_none() && values.len() != items.len()) {
                    false
                } else {
                    let mut matches = true;
                    for (idx, item_pattern) in items.iter().enumerate() {
                        if !apply_pattern_bindings(item_pattern, &values[idx], env)? {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        if let Some(tail_pattern) = tail {
                            let tail_values = values[items.len()..].to_vec();
                            apply_pattern_bindings(
                                tail_pattern,
                                &StaticForValue::List(tail_values),
                                env,
                            )?
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                }
            } else {
                false
            }
        }
        IrPattern::Map { .. } => {
            return Err(StaticForEvalIssue::Unsupported(
                "for helper does not support map patterns".to_string(),
            ));
        }
        IrPattern::Bitstring { .. } => {
            return Err(StaticForEvalIssue::Unsupported(
                "for helper does not support bitstring patterns".to_string(),
            ));
        }
    };

    if matched {
        Ok(true)
    } else {
        *env = snapshot;
        Ok(false)
    }
}

fn pop_static_for_value(
    stack: &mut Vec<StaticForValue>,
    context: &str,
) -> Result<StaticForValue, StaticForEvalIssue> {
    stack.pop().ok_or_else(|| {
        StaticForEvalIssue::Unsupported(format!("for helper stack underflow for {context}"))
    })
}

fn pop_static_for_int(
    stack: &mut Vec<StaticForValue>,
    context: &str,
) -> Result<i64, StaticForEvalIssue> {
    match pop_static_for_value(stack, context)? {
        StaticForValue::Int(value) => Ok(value),
        other => Err(StaticForEvalIssue::Runtime(format!(
            "for arithmetic expects int {context}, found {}",
            other.kind_label()
        ))),
    }
}
