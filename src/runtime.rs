use crate::ir::{IrCallTarget, IrCaseBranch, IrForGenerator, IrOp, IrPattern, IrProgram};
use crate::native_runtime;
use std::collections::HashMap;
use std::fmt;

const ENTRYPOINT: &str = "Demo.run";
const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeClosure {
    params: Vec<String>,
    ops: Vec<IrOp>,
    env: HashMap<String, RuntimeValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Int(i64),
    Float(String),
    Bool(bool),
    Nil,
    String(String),
    Atom(String),
    ResultOk(Box<RuntimeValue>),
    ResultErr(Box<RuntimeValue>),
    Tuple(Box<RuntimeValue>, Box<RuntimeValue>),
    Map(Vec<(RuntimeValue, RuntimeValue)>),
    Keyword(Vec<(RuntimeValue, RuntimeValue)>),
    List(Vec<RuntimeValue>),
    Range(i64, i64),
    SteppedRange(i64, i64, i64),
    Closure(Box<RuntimeClosure>),
}

impl RuntimeValue {
    pub fn render(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Nil => "nil".to_string(),
            Self::String(value) => format!("\"{}\"", value),
            Self::Atom(value) => format!(":{value}"),
            Self::ResultOk(value) => format!("ok({})", value.render()),
            Self::ResultErr(value) => format!("err({})", value.render()),
            Self::Tuple(left, right) => format!("{{{}, {}}}", left.render(), right.render()),
            Self::Map(entries) => {
                let rendered_entries = entries
                    .iter()
                    .map(|(key, value)| format!("{} => {}", key.render(), value.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("%{{{rendered_entries}}}")
            }
            Self::Keyword(entries) => {
                let rendered_entries = entries
                    .iter()
                    .map(|(key, value)| {
                        let rendered_key = match key {
                            RuntimeValue::Atom(atom) => atom.clone(),
                            _ => key.render(),
                        };
                        format!("{rendered_key}: {}", value.render())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{rendered_entries}]")
            }
            Self::List(items) => {
                let items: Vec<String> = items.iter().map(|item| item.render()).collect();
                format!("[{}]", items.join(", "))
            }
            Self::Range(start, end) => format!("{}..{}", start, end),
            Self::SteppedRange(start, end, step) => format!("{}..{}//{}", start, end, step),
            Self::Closure(closure) => format!("#Function<{}>", closure.params.len()),
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Bool(_) => "bool",
            Self::Nil => "nil",
            Self::String(_) => "string",
            Self::Atom(_) => "atom",
            Self::ResultOk(_) | Self::ResultErr(_) => "result",
            Self::Tuple(_, _) => "tuple",
            Self::Map(_) => "map",
            Self::Keyword(_) => "keyword",
            Self::List(_) => "list",
            Self::Range(_, _) => "range",
            Self::SteppedRange(_, _, _) => "stepped_range",
            Self::Closure(_) => "function",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
    offset: Option<usize>,
    pub raised_value: Option<RuntimeValue>,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset: None,
            raised_value: None,
        }
    }

    fn at_offset(message: impl Into<String>, offset: usize) -> Self {
        Self {
            message: message.into(),
            offset: Some(offset),
            raised_value: None,
        }
    }

    fn raised(value: RuntimeValue, offset: usize) -> Self {
        let message = extract_raised_message(&value);
        Self {
            message,
            offset: Some(offset),
            raised_value: Some(value),
        }
    }
}

fn extract_raised_message(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Atom(a) => a.clone(),
        RuntimeValue::Map(entries) => map_lookup_atom(entries, "message")
            .map(|message| match message {
                RuntimeValue::String(text) => text.clone(),
                other => other.render(),
            })
            .unwrap_or_else(|| "exception raised".to_string()),
        _ => "exception raised".to_string(),
    }
}

fn map_lookup_atom<'a>(
    entries: &'a [(RuntimeValue, RuntimeValue)],
    key: &str,
) -> Option<&'a RuntimeValue> {
    entries.iter().find_map(|(entry_key, value)| {
        if let RuntimeValue::Atom(atom) = entry_key {
            (atom == key).then_some(value)
        } else {
            None
        }
    })
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "{} at offset {}", self.message, offset)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for RuntimeError {}

impl RuntimeError {
    pub fn offset(&self) -> Option<usize> {
        self.offset
    }
}

pub fn evaluate_entrypoint(program: &IrProgram) -> Result<RuntimeValue, RuntimeError> {
    evaluate_function(program, ENTRYPOINT, &[], 0)
}

pub fn evaluate_named_function(
    program: &IrProgram,
    function_name: &str,
) -> Result<RuntimeValue, RuntimeError> {
    evaluate_function(program, function_name, &[], 0)
}

fn evaluate_function(
    program: &IrProgram,
    function_name: &str,
    args: &[RuntimeValue],
    call_offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let all_candidates = program
        .functions
        .iter()
        .filter(|function| function.name == function_name)
        .collect::<Vec<_>>();

    if all_candidates.is_empty() {
        return Err(RuntimeError::new(format!(
            "missing runtime function: {function_name}"
        )));
    }

    let arity_candidates = all_candidates
        .iter()
        .copied()
        .filter(|function| function.params.len() == args.len())
        .collect::<Vec<_>>();

    if arity_candidates.is_empty() {
        let expected_arity = all_candidates
            .first()
            .map(|function| function.params.len())
            .unwrap_or(0);

        return Err(RuntimeError::new(format!(
            "arity mismatch for runtime function {function_name}: expected {} args, found {}",
            expected_arity,
            args.len()
        )));
    }

    let mut fallback_guard_offset = None;

    for function in arity_candidates {
        let mut env = HashMap::new();

        if let Some(patterns) = &function.param_patterns {
            let mut matched = true;
            for (pattern, arg) in patterns.iter().zip(args.iter()) {
                let mut bindings = HashMap::new();
                if !match_pattern(arg, pattern, &env, &mut bindings) {
                    matched = false;
                    break;
                }
                env.extend(bindings);
            }

            if !matched {
                continue;
            }
        } else {
            for (param, arg) in function.params.iter().zip(args.iter()) {
                env.insert(param.clone(), arg.clone());
            }
        }

        if let Some(guard_ops) = &function.guard_ops {
            let guard_passed = evaluate_guard_ops(program, guard_ops, &mut env)?;
            if !guard_passed {
                fallback_guard_offset = guard_ops.first().map(ir_op_offset);
                continue;
            }
        }

        let mut stack: Vec<RuntimeValue> = Vec::new();

        if let Some(ret) = evaluate_ops(program, &function.ops, &mut env, &mut stack)? {
            return Ok(ret);
        }

        return Err(RuntimeError::new(format!(
            "runtime function ended without return: {function_name}"
        )));
    }

    Err(RuntimeError::at_offset(
        format!("no function clause matching {function_name}"),
        fallback_guard_offset.unwrap_or(call_offset),
    ))
}

#[path = "runtime_try.rs"]
mod try_helper;

#[path = "runtime_eval.rs"]
mod eval;
pub(crate) use eval::evaluate_ops;

#[path = "runtime_for.rs"]
mod for_comp;
use for_comp::*;

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
