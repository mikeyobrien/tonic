use crate::ir::IrPattern;
use crate::runtime::RuntimeValue;
use std::collections::HashMap;

pub(crate) fn select_case_branch(
    subject: &RuntimeValue,
    branches: &[IrPattern],
    env: &HashMap<String, RuntimeValue>,
) -> Option<(usize, HashMap<String, RuntimeValue>)> {
    branches.iter().enumerate().find_map(|(index, pattern)| {
        let mut bindings = HashMap::new();
        if match_pattern(subject, pattern, env, &mut bindings) {
            Some((index, bindings))
        } else {
            None
        }
    })
}

pub(crate) fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    match pattern {
        IrPattern::Wildcard => true,
        IrPattern::Bind { name } => {
            if let Some(existing) = bindings.get(name) {
                return existing == value;
            }

            bindings.insert(name.clone(), value.clone());
            true
        }
        IrPattern::Pin { name } => bindings
            .get(name)
            .or_else(|| env.get(name))
            .is_some_and(|pinned| pinned == value),
        IrPattern::Integer { value: p_val } => match value {
            RuntimeValue::Int(v) => v == p_val,
            _ => false,
        },
        IrPattern::Bool { value: p_val } => match value {
            RuntimeValue::Bool(v) => v == p_val,
            _ => false,
        },
        IrPattern::Nil => matches!(value, RuntimeValue::Nil),
        IrPattern::String { value: p_val } => match value {
            RuntimeValue::String(v) => v == p_val,
            _ => false,
        },
        IrPattern::Atom { value: p_val } => match value {
            RuntimeValue::Atom(v) => v == p_val,
            _ => false,
        },
        IrPattern::Tuple { items } => match value {
            RuntimeValue::Tuple(left, right) if items.len() == 2 => {
                match_pattern(left, &items[0], env, bindings)
                    && match_pattern(right, &items[1], env, bindings)
            }
            _ => false,
        },
        IrPattern::List { items, tail } => match value {
            RuntimeValue::List(values) => {
                if values.len() < items.len() {
                    return false;
                }

                let prefix_matches = values.iter().take(items.len()).zip(items.iter()).all(
                    |(candidate, candidate_pattern)| {
                        match_pattern(candidate, candidate_pattern, env, bindings)
                    },
                );

                if !prefix_matches {
                    return false;
                }

                if let Some(tail_pattern) = tail {
                    let tail_values = values[items.len()..].to_vec();
                    match_pattern(
                        &RuntimeValue::List(tail_values),
                        tail_pattern,
                        env,
                        bindings,
                    )
                } else {
                    values.len() == items.len()
                }
            }
            _ => false,
        },
        IrPattern::Map { entries } => match value {
            RuntimeValue::Map(values) => {
                for entry in entries {
                    let mut entry_matched = false;

                    for (candidate_key, candidate_value) in values {
                        let mut candidate_bindings = bindings.clone();
                        if match_pattern(candidate_key, &entry.key, env, &mut candidate_bindings)
                            && match_pattern(
                                candidate_value,
                                &entry.value,
                                env,
                                &mut candidate_bindings,
                            )
                        {
                            *bindings = candidate_bindings;
                            entry_matched = true;
                            break;
                        }
                    }

                    if !entry_matched {
                        return false;
                    }
                }

                true
            }
            _ => false,
        },
    }
}
