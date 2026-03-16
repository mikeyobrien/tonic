use super::{StaticForEvalIssue, StaticForValue};
use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern};
use std::collections::BTreeMap;

pub(super) fn evaluate_static_for_ops(
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

pub(super) fn apply_pattern_bindings(
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

pub(super) fn pop_static_for_value(
    stack: &mut Vec<StaticForValue>,
    context: &str,
) -> Result<StaticForValue, StaticForEvalIssue> {
    stack.pop().ok_or_else(|| {
        StaticForEvalIssue::Unsupported(format!("for helper stack underflow for {context}"))
    })
}

pub(super) fn pop_static_for_int(
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
