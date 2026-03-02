use crate::ir::{IrCallTarget, IrForGenerator, IrOp, IrPattern, IrProgram};
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
            Self::SteppedRange(start, end, step) => format!("{}..{}//{}" , start, end, step),
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

fn evaluate_ops(
    program: &IrProgram,
    ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    stack: &mut Vec<RuntimeValue>,
) -> Result<Option<RuntimeValue>, RuntimeError> {
    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => stack.push(RuntimeValue::Int(*value)),
            IrOp::ConstFloat { value, .. } => stack.push(RuntimeValue::Float(value.clone())),
            IrOp::ConstBool { value, .. } => stack.push(RuntimeValue::Bool(*value)),
            IrOp::ConstNil { .. } => stack.push(RuntimeValue::Nil),
            IrOp::ConstString { value, .. } => stack.push(RuntimeValue::String(value.clone())),
            IrOp::ToString { offset } => {
                let value = pop_value(stack, *offset, "to_string")?;
                let str_value = match value {
                    RuntimeValue::String(s) => s,
                    RuntimeValue::Int(i) => i.to_string(),
                    RuntimeValue::Float(f) => f.clone(),
                    RuntimeValue::Bool(b) => b.to_string(),
                    RuntimeValue::Nil => String::new(),
                    RuntimeValue::Atom(a) => a,
                    _ => {
                        return Err(RuntimeError::at_offset(
                            "cannot interpolate complex value".to_string(),
                            *offset,
                        ))
                    }
                };
                stack.push(RuntimeValue::String(str_value));
            }
            IrOp::ConstAtom { value, .. } => stack.push(RuntimeValue::Atom(value.clone())),
            IrOp::LoadVariable { name, offset } => {
                let value = env.get(name).ok_or_else(|| {
                    RuntimeError::at_offset(format!("undefined variable: {name}"), *offset)
                })?;
                stack.push(value.clone());
            }
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                let value = evaluate_call(program, callee, stack, *argc, *offset)?;
                stack.push(value);
            }
            IrOp::MakeClosure { params, ops, .. } => {
                stack.push(RuntimeValue::Closure(Box::new(RuntimeClosure {
                    params: params.clone(),
                    ops: ops.clone(),
                    env: env.clone(),
                })));
            }
            IrOp::CallValue { argc, offset } => {
                let value = evaluate_call_value(program, stack, *argc, *offset)?;
                stack.push(value);
            }
            IrOp::Not { offset } => {
                let value = pop_value(stack, *offset, "not")?;
                let result = native_runtime::ops::strict_not(value, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::Bang { offset } => {
                let value = pop_value(stack, *offset, "!")?;
                stack.push(native_runtime::ops::truthy_bang(value));
            }
            IrOp::AndAnd { right_ops, offset } => {
                let value = pop_value(stack, *offset, "&&")?;
                let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));
                if truthy {
                    let mut child_stack = Vec::new();
                    evaluate_ops(program, right_ops, env, &mut child_stack)?;
                    stack.push(pop_value(&mut child_stack, *offset, "&&")?);
                } else {
                    stack.push(value);
                }
            }
            IrOp::OrOr { right_ops, offset } => {
                let value = pop_value(stack, *offset, "||")?;
                let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));
                if truthy {
                    stack.push(value);
                } else {
                    let mut child_stack = Vec::new();
                    evaluate_ops(program, right_ops, env, &mut child_stack)?;
                    stack.push(pop_value(&mut child_stack, *offset, "||")?);
                }
            }
            IrOp::And { right_ops, offset } => {
                let left = pop_value(stack, *offset, "and")?;
                match left {
                    RuntimeValue::Bool(true) => {
                        let mut child_stack = Vec::new();
                        evaluate_ops(program, right_ops, env, &mut child_stack)?;
                        stack.push(pop_value(&mut child_stack, *offset, "and")?);
                    }
                    RuntimeValue::Bool(false) => {
                        stack.push(RuntimeValue::Bool(false));
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::Or { right_ops, offset } => {
                let left = pop_value(stack, *offset, "or")?;
                match left {
                    RuntimeValue::Bool(false) => {
                        let mut child_stack = Vec::new();
                        evaluate_ops(program, right_ops, env, &mut child_stack)?;
                        stack.push(pop_value(&mut child_stack, *offset, "or")?);
                    }
                    RuntimeValue::Bool(true) => {
                        stack.push(RuntimeValue::Bool(true));
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::Concat { offset } => {
                let right = pop_value(stack, *offset, "<>")?;
                let left = pop_value(stack, *offset, "<>")?;
                let result = native_runtime::ops::concat(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::In { offset } => {
                let right = pop_value(stack, *offset, "in")?;
                let left = pop_value(stack, *offset, "in")?;
                let result = native_runtime::ops::in_operator(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::PlusPlus { offset } => {
                let right = pop_value(stack, *offset, "++")?;
                let left = pop_value(stack, *offset, "++")?;
                let result = native_runtime::ops::list_concat(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::MinusMinus { offset } => {
                let right = pop_value(stack, *offset, "--")?;
                let left = pop_value(stack, *offset, "--")?;
                let result = native_runtime::ops::list_subtract(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::Range { offset } => {
                let right = pop_value(stack, *offset, "range")?;
                let left = pop_value(stack, *offset, "range")?;
                let result = native_runtime::ops::range(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::NotIn { offset } => {
                let right = pop_value(stack, *offset, "not in")?;
                let left = pop_value(stack, *offset, "not in")?;
                let result = native_runtime::ops::in_operator(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                // Negate the in result
                let negated = match result {
                    RuntimeValue::Bool(b) => RuntimeValue::Bool(!b),
                    other => other,
                };
                stack.push(negated);
            }
            IrOp::BitwiseAnd { offset } => {
                let right = pop_value(stack, *offset, "&&&")?;
                let left = pop_value(stack, *offset, "&&&")?;
                let result = native_runtime::ops::bitwise_and(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::BitwiseOr { offset } => {
                let right = pop_value(stack, *offset, "|||")?;
                let left = pop_value(stack, *offset, "|||")?;
                let result = native_runtime::ops::bitwise_or(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::BitwiseXor { offset } => {
                let right = pop_value(stack, *offset, "^^^")?;
                let left = pop_value(stack, *offset, "^^^")?;
                let result = native_runtime::ops::bitwise_xor(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::BitwiseNot { offset } => {
                let value = pop_value(stack, *offset, "~~~")?;
                let result = native_runtime::ops::bitwise_not(value, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::BitwiseShiftLeft { offset } => {
                let right = pop_value(stack, *offset, "<<<")?;
                let left = pop_value(stack, *offset, "<<<")?;
                let result = native_runtime::ops::bitwise_shift_left(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::BitwiseShiftRight { offset } => {
                let right = pop_value(stack, *offset, ">>>")?;
                let left = pop_value(stack, *offset, ">>>")?;
                let result = native_runtime::ops::bitwise_shift_right(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::SteppedRange { offset } => {
                // Stack: ... range step
                let step = pop_value(stack, *offset, "stepped range step")?;
                let range = pop_value(stack, *offset, "stepped range range")?;
                let result = native_runtime::ops::stepped_range(range, step, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::AddInt { offset } => {
                let right = pop_value(stack, *offset, "+")?;
                let left = pop_value(stack, *offset, "+")?;
                let result = native_runtime::ops::add_int(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::SubInt { offset } => {
                let right = pop_value(stack, *offset, "-")?;
                let left = pop_value(stack, *offset, "-")?;
                let result = native_runtime::ops::sub_int(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::MulInt { offset } => {
                let right = pop_value(stack, *offset, "*")?;
                let left = pop_value(stack, *offset, "*")?;
                let result = native_runtime::ops::mul_int(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::DivInt { offset } => {
                let right = pop_value(stack, *offset, "/")?;
                let left = pop_value(stack, *offset, "/")?;
                let result = native_runtime::ops::div_int(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::CmpInt { kind, offset } => {
                let right = pop_value(stack, *offset, "cmp")?;
                let left = pop_value(stack, *offset, "cmp")?;
                let result = native_runtime::ops::cmp_int(*kind, left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::Match { pattern, offset } => {
                let value = pop_value(stack, *offset, "match")?;
                let mut bindings = HashMap::new();

                if !match_pattern(&value, pattern, env, &mut bindings) {
                    return Err(RuntimeError::at_offset(
                        format!("no match of right hand side value: {}", value.render()),
                        *offset,
                    ));
                }

                for (name, bound_value) in bindings {
                    env.insert(name, bound_value);
                }

                stack.push(value);
            }
            IrOp::Return { offset } => {
                return Ok(Some(pop_value(stack, *offset, "return")?));
            }
            IrOp::Question { offset } => {
                let value = pop_value(stack, *offset, "question")?;

                match value {
                    RuntimeValue::ResultOk(inner) => stack.push(*inner),
                    RuntimeValue::ResultErr(inner) => {
                        return Ok(Some(RuntimeValue::ResultErr(inner)));
                    }
                    other => {
                        return Err(RuntimeError::at_offset(
                            format!(
                                "question expects result value, found {}",
                                other.kind_label()
                            ),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::Case { branches, offset } => {
                let subject = pop_value(stack, *offset, "case subject")?;

                if branches.iter().all(|branch| branch.guard_ops.is_none()) {
                    let patterns = branches
                        .iter()
                        .map(|branch| branch.pattern.clone())
                        .collect::<Vec<_>>();

                    if let Some((selected_index, bindings)) =
                        native_runtime::pattern::select_case_branch(&subject, &patterns, env)
                    {
                        let mut branch_env = env.clone();
                        for (name, value) in bindings {
                            branch_env.insert(name, value);
                        }

                        if let Some(ret) = evaluate_ops(
                            program,
                            &branches[selected_index].ops,
                            &mut branch_env,
                            stack,
                        )? {
                            return Ok(Some(ret));
                        }
                    } else {
                        return Err(RuntimeError::at_offset("no case clause matching", *offset));
                    }

                    continue;
                }

                let mut matched = false;
                for branch in branches {
                    let mut bindings = HashMap::new();
                    if !match_pattern(&subject, &branch.pattern, env, &mut bindings) {
                        continue;
                    }

                    let mut branch_env = env.clone();
                    for (name, value) in bindings {
                        branch_env.insert(name, value);
                    }

                    if let Some(guard_ops) = &branch.guard_ops {
                        let guard_passed = evaluate_guard_ops(program, guard_ops, &mut branch_env)?;
                        if !guard_passed {
                            continue;
                        }
                    }

                    matched = true;

                    if let Some(ret) = evaluate_ops(program, &branch.ops, &mut branch_env, stack)? {
                        return Ok(Some(ret));
                    }
                    break;
                }

                if !matched {
                    return Err(RuntimeError::at_offset("no case clause matching", *offset));
                }
            }
            IrOp::Try {
                body_ops,
                rescue_branches,
                catch_branches,
                after_ops,
                offset: _,
            } => {
                let mut try_env = env.clone();
                let mut try_stack = Vec::new();

                let mut early_return = None;
                let mut final_err = None;

                match evaluate_ops(program, body_ops, &mut try_env, &mut try_stack) {
                    Ok(ret) => {
                        if let Some(v) = ret {
                            early_return = Some(v);
                        } else if let Some(v) = try_stack.pop() {
                            stack.push(v);
                        } else {
                            stack.push(RuntimeValue::Nil);
                        }
                    }
                    Err(err) => {
                        let err_val = err
                            .raised_value
                            .clone()
                            .unwrap_or_else(|| RuntimeValue::String(err.message.clone()));

                        let mut handled = false;
                        for branch in rescue_branches {
                            let mut bindings = HashMap::new();
                            if !match_pattern(&err_val, &branch.pattern, env, &mut bindings) {
                                continue;
                            }

                            let mut branch_env = env.clone();
                            for (k, v) in bindings {
                                branch_env.insert(k, v);
                            }

                            if let Some(guard_ops) = &branch.guard_ops {
                                let guard_passed =
                                    evaluate_guard_ops(program, guard_ops, &mut branch_env)?;
                                if !guard_passed {
                                    continue;
                                }
                            }

                            let mut branch_stack = Vec::new();
                            match evaluate_ops(
                                program,
                                &branch.ops,
                                &mut branch_env,
                                &mut branch_stack,
                            ) {
                                Ok(ret) => {
                                    if let Some(v) = ret {
                                        early_return = Some(v);
                                    } else {
                                        let result = branch_stack.pop().unwrap_or_else(|| {
                                            RuntimeValue::Atom("ok".to_string())
                                        });
                                        stack.push(result);
                                    }
                                }
                                Err(e) => final_err = Some(e),
                            }
                            handled = true;
                            break;
                        }

                        if !handled {
                            for branch in catch_branches {
                                let mut bindings = HashMap::new();
                                if !match_pattern(&err_val, &branch.pattern, env, &mut bindings) {
                                    continue;
                                }

                                let mut branch_env = env.clone();
                                for (k, v) in bindings {
                                    branch_env.insert(k, v);
                                }

                                if let Some(guard_ops) = &branch.guard_ops {
                                    let guard_passed =
                                        evaluate_guard_ops(program, guard_ops, &mut branch_env)?;
                                    if !guard_passed {
                                        continue;
                                    }
                                }

                                let mut branch_stack = Vec::new();
                                match evaluate_ops(
                                    program,
                                    &branch.ops,
                                    &mut branch_env,
                                    &mut branch_stack,
                                ) {
                                    Ok(ret) => {
                                        if let Some(v) = ret {
                                            early_return = Some(v);
                                        } else {
                                            let result = branch_stack.pop().unwrap_or_else(|| {
                                                RuntimeValue::Atom("ok".to_string())
                                            });
                                            stack.push(result);
                                        }
                                    }
                                    Err(e) => final_err = Some(e),
                                }
                                handled = true;
                                break;
                            }
                        }

                        if !handled {
                            final_err = Some(err);
                        }
                    }
                }

                if let Some(after) = after_ops {
                    let mut after_env = env.clone();
                    let mut after_stack = Vec::new();
                    let _ = evaluate_ops(program, after, &mut after_env, &mut after_stack);
                }

                if let Some(err) = final_err {
                    return Err(err);
                }

                if let Some(v) = early_return {
                    return Ok(Some(v));
                }
            }
            IrOp::For {
                generators,
                filter_ops,
                body_ops,
                into_ops,
                offset,
            } => {
                let result = evaluate_for(
                    program,
                    generators,
                    filter_ops,
                    body_ops,
                    into_ops,
                    env,
                    *offset,
                )?;
                stack.push(result);
            }
            IrOp::MakeTuple { offset } => {
                let right = pop_value(stack, *offset, "tuple")?;
                let left = pop_value(stack, *offset, "tuple")?;
                stack.push(RuntimeValue::Tuple(Box::new(left), Box::new(right)));
            }
            IrOp::MakeMap { size, offset } => {
                let mut entries = Vec::new();
                for _ in 0..*size {
                    let value = pop_value(stack, *offset, "map value")?;
                    let key = pop_value(stack, *offset, "map key")?;
                    entries.push((key, value));
                }
                entries.reverse();
                stack.push(RuntimeValue::Map(entries));
            }
            IrOp::MakeKeyword { size, offset } => {
                let mut entries = Vec::new();
                for _ in 0..*size {
                    let value = pop_value(stack, *offset, "keyword value")?;
                    let key = pop_value(stack, *offset, "keyword key")?;
                    entries.push((key, value));
                }
                entries.reverse();
                stack.push(RuntimeValue::Keyword(entries));
            }
            IrOp::MakeList { size, offset } => {
                let mut items = Vec::new();
                for _ in 0..*size {
                    items.push(pop_value(stack, *offset, "list item")?);
                }
                items.reverse();
                stack.push(RuntimeValue::List(items));
            }
            IrOp::Raise { offset } => {
                let value = pop_value(stack, *offset, "raise")?;
                return Err(RuntimeError::raised(value, *offset));
            }
            IrOp::If {
                then_ops,
                else_ops,
                offset,
            } => {
                let value = pop_value(stack, *offset, "if condition")?;
                let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));

                let branch_ops = if truthy { then_ops } else { else_ops };

                let mut branch_env = env.clone();
                if let Some(ret) = evaluate_ops(program, branch_ops, &mut branch_env, stack)? {
                    return Ok(Some(ret));
                }
            }
            IrOp::Unless {
                then_ops,
                else_ops,
                offset,
            } => {
                let value = pop_value(stack, *offset, "unless condition")?;
                let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));

                let branch_ops = if truthy { else_ops } else { then_ops };

                let mut branch_env = env.clone();
                if let Some(ret) = evaluate_ops(program, branch_ops, &mut branch_env, stack)? {
                    return Ok(Some(ret));
                }
            }
            IrOp::Cond { branches, offset } => {
                let mut matched = false;
                for branch in branches {
                    let mut cond_stack = Vec::new();
                    evaluate_ops(program, &branch.condition_ops, env, &mut cond_stack)?;
                    let cond_value = pop_value(&mut cond_stack, *offset, "cond condition")?;
                    let truthy =
                        !matches!(cond_value, RuntimeValue::Nil | RuntimeValue::Bool(false));

                    if truthy {
                        let mut branch_env = env.clone();
                        if let Some(ret) =
                            evaluate_ops(program, &branch.ops, &mut branch_env, stack)?
                        {
                            return Ok(Some(ret));
                        }
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(RuntimeError::at_offset(
                        "no cond clause matched".to_string(),
                        *offset,
                    ));
                }
            }
            IrOp::With {
                match_ops,
                else_branches,
                body_ops,
                offset,
            } => {
                let mut with_env = env.clone();
                let mut short_circuit_val = None;

                for (match_val_ops, pattern) in match_ops {
                    let mut val_stack = Vec::new();
                    evaluate_ops(program, match_val_ops, &mut with_env, &mut val_stack)?;
                    let val = pop_value(&mut val_stack, *offset, "with match")?;

                    let mut bindings = HashMap::new();
                    if !match_pattern(&val, pattern, &with_env, &mut bindings) {
                        // Check else branches
                        let mut else_matched = false;
                        for else_branch in else_branches {
                            let mut else_bindings = HashMap::new();
                            if match_pattern(&val, &else_branch.pattern, env, &mut else_bindings) {
                                let mut else_env = env.clone();
                                else_env.extend(else_bindings);
                                let mut else_stack = Vec::new();
                                if let Some(ret) = evaluate_ops(
                                    program,
                                    &else_branch.ops,
                                    &mut else_env,
                                    &mut else_stack,
                                )? {
                                    return Ok(Some(ret));
                                }
                                let v = else_stack
                                    .pop()
                                    .unwrap_or(RuntimeValue::Atom("ok".to_string()));
                                short_circuit_val = Some(v);
                                else_matched = true;
                                break;
                            }
                        }
                        if !else_matched {
                            short_circuit_val = Some(val);
                        }
                        break;
                    }
                    with_env.extend(bindings);
                }

                if let Some(v) = short_circuit_val {
                    stack.push(v);
                    continue;
                }

                let mut body_stack = Vec::new();
                if let Some(ret) = evaluate_ops(program, body_ops, &mut with_env, &mut body_stack)?
                {
                    return Ok(Some(ret));
                }
                let body_val = body_stack
                    .pop()
                    .unwrap_or(RuntimeValue::Atom("ok".to_string()));
                stack.push(body_val);
            }
            IrOp::MapGet { key, offset } => {
                let map = pop_value(stack, *offset, "map_get")?;

                let entries = match &map {
                    RuntimeValue::Map(e) => e.clone(),
                    RuntimeValue::Keyword(e) => e.clone(),
                    _ => {
                        return Err(RuntimeError::at_offset(
                            format!("map_get on non-map: {}", map.kind_label()),
                            *offset,
                        ));
                    }
                };

                let val = match key.as_str() {
                    _ => map_lookup_atom(&entries, key),
                };

                match val {
                    Some(v) => stack.push(v.clone()),
                    None => {
                        return Err(RuntimeError::at_offset(
                            format!("key not found in map: {key}"),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::MapUpdate { updates, offset } => {
                let map = pop_value(stack, *offset, "map_update")?;

                match map {
                    RuntimeValue::Map(mut entries) => {
                        for (update_key, update_ops) in updates {
                            let mut update_stack = Vec::new();
                            evaluate_ops(program, update_ops, env, &mut update_stack)?;
                            let update_val =
                                pop_value(&mut update_stack, *offset, "map_update value")?;

                            let key_val = RuntimeValue::Atom(update_key.clone());
                            if let Some(existing) =
                                entries.iter_mut().find(|(k, _)| *k == key_val)
                            {
                                existing.1 = update_val;
                            } else {
                                return Err(RuntimeError::at_offset(
                                    format!("key not found in map for update: {update_key}"),
                                    *offset,
                                ));
                            }
                        }
                        stack.push(RuntimeValue::Map(entries));
                    }
                    _ => {
                        return Err(RuntimeError::at_offset(
                            "map_update on non-map".to_string(),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::StructGet { key, offset } => {
                let map = pop_value(stack, *offset, "struct_get")?;

                let entries = match &map {
                    RuntimeValue::Map(e) => e.clone(),
                    _ => {
                        return Err(RuntimeError::at_offset(
                            format!("struct_get on non-map: {}", map.kind_label()),
                            *offset,
                        ));
                    }
                };

                let val = map_lookup_atom(&entries, key);

                match val {
                    Some(v) => stack.push(v.clone()),
                    None => {
                        return Err(RuntimeError::at_offset(
                            format!("key not found in struct: {key}"),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::StructUpdate { updates, offset } => {
                let map = pop_value(stack, *offset, "struct_update")?;

                match map {
                    RuntimeValue::Map(mut entries) => {
                        for (update_key, update_ops) in updates {
                            let mut update_stack = Vec::new();
                            evaluate_ops(program, update_ops, env, &mut update_stack)?;
                            let update_val =
                                pop_value(&mut update_stack, *offset, "struct_update value")?;

                            let key_val = RuntimeValue::Atom(update_key.clone());
                            if let Some(existing) =
                                entries.iter_mut().find(|(k, _)| *k == key_val)
                            {
                                existing.1 = update_val;
                            } else {
                                entries.push((key_val, update_val));
                            }
                        }
                        stack.push(RuntimeValue::Map(entries));
                    }
                    _ => {
                        return Err(RuntimeError::at_offset(
                            "struct_update on non-map".to_string(),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::KeywordGet { key, offset } => {
                let kw = pop_value(stack, *offset, "keyword_get")?;

                let entries = match &kw {
                    RuntimeValue::Keyword(e) => e.clone(),
                    RuntimeValue::List(items) => {
                        // Support keyword list access on plain list
                        items
                            .iter()
                            .filter_map(|item| {
                                if let RuntimeValue::Tuple(k, v) = item {
                                    Some((*k.clone(), *v.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                    _ => {
                        return Err(RuntimeError::at_offset(
                            format!("keyword_get on non-keyword: {}", kw.kind_label()),
                            *offset,
                        ));
                    }
                };

                let val = map_lookup_atom(&entries, key);

                match val {
                    Some(v) => stack.push(v.clone()),
                    None => stack.push(RuntimeValue::Nil),
                }
            }
            IrOp::Eq { offset } => {
                let right = pop_value(stack, *offset, "==")?;
                let left = pop_value(stack, *offset, "==")?;
                stack.push(RuntimeValue::Bool(left == right));
            }
            IrOp::NotEq { offset } => {
                let right = pop_value(stack, *offset, "!=")?;
                let left = pop_value(stack, *offset, "!=")?;
                stack.push(RuntimeValue::Bool(left != right));
            }
            IrOp::Noop => {}
            IrOp::DebugPrint { offset } => {
                let value = pop_value(stack, *offset, "debug_print")?;
                println!("{}", value.render());
                stack.push(value);
            }
        }
    }

    Ok(None)
}

fn ir_op_offset(op: &IrOp) -> usize {
    match op {
        IrOp::ConstInt { offset, .. } => *offset,
        IrOp::ConstFloat { offset, .. } => *offset,
        IrOp::ConstBool { offset, .. } => *offset,
        IrOp::ConstNil { offset } => *offset,
        IrOp::ConstString { offset, .. } => *offset,
        IrOp::ConstAtom { offset, .. } => *offset,
        IrOp::LoadVariable { offset, .. } => *offset,
        IrOp::Call { offset, .. } => *offset,
        IrOp::CallValue { offset, .. } => *offset,
        IrOp::MakeClosure { offset, .. } => *offset,
        IrOp::Return { offset } => *offset,
        IrOp::Not { offset } => *offset,
        IrOp::Bang { offset } => *offset,
        IrOp::AndAnd { offset, .. } => *offset,
        IrOp::OrOr { offset, .. } => *offset,
        IrOp::And { offset, .. } => *offset,
        IrOp::Or { offset, .. } => *offset,
        IrOp::Concat { offset } => *offset,
        IrOp::In { offset } => *offset,
        IrOp::PlusPlus { offset } => *offset,
        IrOp::MinusMinus { offset } => *offset,
        IrOp::Range { offset } => *offset,
        IrOp::NotIn { offset } => *offset,
        IrOp::BitwiseAnd { offset } => *offset,
        IrOp::BitwiseOr { offset } => *offset,
        IrOp::BitwiseXor { offset } => *offset,
        IrOp::BitwiseNot { offset } => *offset,
        IrOp::BitwiseShiftLeft { offset } => *offset,
        IrOp::BitwiseShiftRight { offset } => *offset,
        IrOp::SteppedRange { offset } => *offset,
        IrOp::AddInt { offset } => *offset,
        IrOp::SubInt { offset } => *offset,
        IrOp::MulInt { offset } => *offset,
        IrOp::DivInt { offset } => *offset,
        IrOp::CmpInt { offset, .. } => *offset,
        IrOp::Match { offset, .. } => *offset,
        IrOp::Question { offset } => *offset,
        IrOp::Case { offset, .. } => *offset,
        IrOp::Try { .. } => 0,
        IrOp::For { offset, .. } => *offset,
        IrOp::MakeTuple { offset } => *offset,
        IrOp::MakeMap { offset, .. } => *offset,
        IrOp::MakeKeyword { offset, .. } => *offset,
        IrOp::MakeList { offset, .. } => *offset,
        IrOp::Raise { offset } => *offset,
        IrOp::If { offset, .. } => *offset,
        IrOp::Unless { offset, .. } => *offset,
        IrOp::Cond { offset, .. } => *offset,
        IrOp::With { offset, .. } => *offset,
        IrOp::MapGet { offset, .. } => *offset,
        IrOp::MapUpdate { offset, .. } => *offset,
        IrOp::StructGet { offset, .. } => *offset,
        IrOp::StructUpdate { offset, .. } => *offset,
        IrOp::KeywordGet { offset, .. } => *offset,
        IrOp::Eq { offset } => *offset,
        IrOp::NotEq { offset } => *offset,
        IrOp::ToString { offset } => *offset,
        IrOp::Noop => 0,
        IrOp::DebugPrint { offset } => *offset,
    }
}

fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    call_offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();

    match callee {
        IrCallTarget::Function(name) => evaluate_function(program, name, &args, call_offset),
        IrCallTarget::Native(name) => {
            native_runtime::call(name, &args, call_offset).map_err(map_native_runtime_error)
        }
    }
}

fn evaluate_call_value(
    program: &IrProgram,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    call_offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();
    let callee = stack.pop().ok_or_else(|| RuntimeError::at_offset("call_value: empty stack", call_offset))?;

    match callee {
        RuntimeValue::Closure(closure) => {
            let mut env = closure.env.clone();
            for (param, arg) in closure.params.iter().zip(args.iter()) {
                env.insert(param.clone(), arg.clone());
            }
            let mut stack = Vec::new();
            if let Some(ret) = evaluate_ops(program, &closure.ops, &mut env, &mut stack)? {
                return Ok(ret);
            }
            Ok(stack.pop().unwrap_or(RuntimeValue::Nil))
        }
        _ => Err(RuntimeError::at_offset(
            format!("call_value: expected closure, found {}", callee.kind_label()),
            call_offset,
        )),
    }
}

fn evaluate_guard_ops(
    program: &IrProgram,
    guard_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
) -> Result<bool, RuntimeError> {
    let mut guard_stack = Vec::new();
    evaluate_ops(program, guard_ops, env, &mut guard_stack)?;
    let guard_result = guard_stack.pop().unwrap_or(RuntimeValue::Bool(false));
    Ok(matches!(guard_result, RuntimeValue::Bool(true)))
}

fn evaluate_for(
    program: &IrProgram,
    generators: &[IrForGenerator],
    filter_ops: &Option<Vec<IrOp>>,
    body_ops: &[IrOp],
    into_ops: &Option<Vec<IrOp>>,
    env: &HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let mut items: Vec<RuntimeValue> = Vec::new();
    evaluate_for_recursive(
        program,
        generators,
        0,
        filter_ops,
        body_ops,
        env,
        offset,
        &mut items,
    )?;

    if let Some(into) = into_ops {
        let mut into_env = env.clone();
        into_env.insert(
            FOR_REDUCE_ACC_BINDING.to_string(),
            RuntimeValue::List(items.clone()),
        );
        let mut into_stack = Vec::new();
        evaluate_ops(program, into, &mut into_env, &mut into_stack)?;
        return Ok(into_stack.pop().unwrap_or(RuntimeValue::Nil));
    }

    Ok(RuntimeValue::List(items))
}

fn evaluate_for_recursive(
    program: &IrProgram,
    generators: &[IrForGenerator],
    depth: usize,
    filter_ops: &Option<Vec<IrOp>>,
    body_ops: &[IrOp],
    env: &HashMap<String, RuntimeValue>,
    offset: usize,
    items: &mut Vec<RuntimeValue>,
) -> Result<(), RuntimeError> {
    if depth >= generators.len() {
        // Apply filter if any
        if let Some(filter) = filter_ops {
            let mut filter_env = env.clone();
            let mut filter_stack = Vec::new();
            evaluate_ops(program, filter, &mut filter_env, &mut filter_stack)?;
            let filter_result = filter_stack.pop().unwrap_or(RuntimeValue::Bool(false));
            if matches!(filter_result, RuntimeValue::Nil | RuntimeValue::Bool(false)) {
                return Ok(());
            }
        }

        // Execute body
        let mut body_env = env.clone();
        let mut body_stack = Vec::new();
        evaluate_ops(program, body_ops, &mut body_env, &mut body_stack)?;
        if let Some(val) = body_stack.pop() {
            items.push(val);
        }
        return Ok(());
    }

    let generator = &generators[depth];
    let mut gen_env = env.clone();
    let mut gen_stack = Vec::new();
    evaluate_ops(program, &generator.collection_ops, &mut gen_env, &mut gen_stack)?;
    let collection = gen_stack
        .pop()
        .ok_or_else(|| RuntimeError::at_offset("for: empty collection", offset))?;

    let items_to_iterate = match collection {
        RuntimeValue::List(items) => items,
        RuntimeValue::Range(start, end) => {
            if start <= end {
                (start..=end).map(RuntimeValue::Int).collect()
            } else {
                vec![]
            }
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            let mut result = vec![];
            if step > 0 {
                let mut current = start;
                while current <= end {
                    result.push(RuntimeValue::Int(current));
                    current += step;
                }
            } else if step < 0 {
                let mut current = start;
                while current >= end {
                    result.push(RuntimeValue::Int(current));
                    current += step;
                }
            }
            result
        }
        other => {
            return Err(RuntimeError::at_offset(
                format!("for: expected list or range, found {}", other.kind_label()),
                offset,
            ));
        }
    };

    for item in items_to_iterate {
        let mut iter_env = env.clone();
        let mut bindings = HashMap::new();
        if match_pattern(&item, &generator.pattern, &iter_env, &mut bindings) {
            iter_env.extend(bindings);
            evaluate_for_recursive(
                program,
                generators,
                depth + 1,
                filter_ops,
                body_ops,
                &iter_env,
                offset,
                items,
            )?;
        }
    }

    Ok(())
}

fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    match pattern {
        IrPattern::Wildcard => true,
        IrPattern::Int(n) => matches!(value, RuntimeValue::Int(v) if v == n),
        IrPattern::Float(f) => matches!(value, RuntimeValue::Float(v) if v == f),
        IrPattern::Bool(b) => matches!(value, RuntimeValue::Bool(v) if v == b),
        IrPattern::Nil => matches!(value, RuntimeValue::Nil),
        IrPattern::String(s) => matches!(value, RuntimeValue::String(v) if v == s),
        IrPattern::Atom(a) => matches!(value, RuntimeValue::Atom(v) if v == a),
        IrPattern::Bind(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        IrPattern::Pin(name) => {
            if let Some(pinned) = env.get(name) {
                value == pinned
            } else {
                false
            }
        }
        IrPattern::Tuple(left_pat, right_pat) => {
            if let RuntimeValue::Tuple(left_val, right_val) = value {
                match_pattern(left_val, left_pat, env, bindings)
                    && match_pattern(right_val, right_pat, env, bindings)
            } else {
                false
            }
        }
        IrPattern::Map(pattern_entries) => {
            if let RuntimeValue::Map(map_entries) = value {
                for (key, val_pattern) in pattern_entries {
                    let atom_key = RuntimeValue::Atom(key.clone());
                    let found = map_entries.iter().find(|(k, _)| *k == atom_key);
                    match found {
                        Some((_, map_val)) => {
                            if !match_pattern(map_val, val_pattern, env, bindings) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            } else {
                false
            }
        }
        IrPattern::Struct(pattern_entries) => {
            if let RuntimeValue::Map(map_entries) = value {
                for (key, val_pattern) in pattern_entries {
                    let atom_key = RuntimeValue::Atom(key.clone());
                    let found = map_entries.iter().find(|(k, _)| *k == atom_key);
                    match found {
                        Some((_, map_val)) => {
                            if !match_pattern(map_val, val_pattern, env, bindings) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            } else {
                false
            }
        }
        IrPattern::List(patterns) => {
            if let RuntimeValue::List(items) = value {
                if patterns.is_empty() {
                    return items.is_empty();
                }
                if items.len() != patterns.len() {
                    return false;
                }
                patterns
                    .iter()
                    .zip(items.iter())
                    .all(|(p, v)| match_pattern(v, p, env, bindings))
            } else {
                false
            }
        }
        IrPattern::ListCons(head_pattern, tail_pattern) => {
            if let RuntimeValue::List(items) = value {
                if items.is_empty() {
                    return false;
                }
                let head = &items[0];
                let tail = RuntimeValue::List(items[1..].to_vec());
                match_pattern(head, head_pattern, env, bindings)
                    && match_pattern(&tail, tail_pattern, env, bindings)
            } else {
                false
            }
        }
        IrPattern::ResultOk(inner_pattern) => {
            if let RuntimeValue::ResultOk(inner_val) = value {
                match_pattern(inner_val, inner_pattern, env, bindings)
            } else {
                false
            }
        }
        IrPattern::ResultErr(inner_pattern) => {
            if let RuntimeValue::ResultErr(inner_val) = value {
                match_pattern(inner_val, inner_pattern, env, bindings)
            } else {
                false
            }
        }
    }
}

fn pop_value(
    stack: &mut Vec<RuntimeValue>,
    offset: usize,
    context: &str,
) -> Result<RuntimeValue, RuntimeError> {
    stack.pop().ok_or_else(|| {
        RuntimeError::at_offset(format!("empty stack in context: {context}"), offset)
    })
}

fn map_native_runtime_error(
    err: native_runtime::NativeRuntimeError,
) -> RuntimeError {
    match err.raised_value {
        Some(val) => RuntimeError::raised(val, err.offset),
        None => RuntimeError::at_offset(err.message, err.offset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler;

    fn compile_and_run(source: &str) -> Result<RuntimeValue, RuntimeError> {
        let program = compiler::compile(source).expect("compile error");
        evaluate_entrypoint(&program)
    }

    fn compile_and_run_named(
        source: &str,
        function_name: &str,
    ) -> Result<RuntimeValue, RuntimeError> {
        let program = compiler::compile(source).expect("compile error");
        evaluate_named_function(&program, function_name)
    }

    #[test]
    fn test_integer_literal() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    42\n  end\nend"),
            Ok(RuntimeValue::Int(42))
        );
    }

    #[test]
    fn test_boolean_literal() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    true\n  end\nend"),
            Ok(RuntimeValue::Bool(true))
        );
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    \"hello\"\n  end\nend"),
            Ok(RuntimeValue::String("hello".to_string()))
        );
    }

    #[test]
    fn test_nil_literal() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    nil\n  end\nend"),
            Ok(RuntimeValue::Nil)
        );
    }

    #[test]
    fn test_atom_literal() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    :ok\n  end\nend"),
            Ok(RuntimeValue::Atom("ok".to_string()))
        );
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(
            compile_and_run("defmodule Demo do\n  def run do\n    1 + 2 * 3\n  end\nend"),
            Ok(RuntimeValue::Int(7))
        );
    }

    #[test]
    fn test_function_call() {
        assert_eq!(
            compile_and_run(
                "defmodule Demo do\n  def run do\n    double(21)\n  end\n  def double(x) do\n    x * 2\n  end\nend"
            ),
            Ok(RuntimeValue::Int(42))
        );
    }

    #[test]
    fn test_pattern_match() {
        assert_eq!(
            compile_and_run(
                "defmodule Demo do\n  def run do\n    {:ok, value} = {:ok, 42}\n    value\n  end\nend"
            ),
            Ok(RuntimeValue::Int(42))
        );
    }

    #[test]
    fn test_case_expression() {
        assert_eq!(
            compile_and_run(
                "defmodule Demo do\n  def run do\n    case :ok do\n      :ok -> 1\n      :error -> 2\n    end\n  end\nend"
            ),
            Ok(RuntimeValue::Int(1))
        );
    }

    #[test]
    fn test_list_operations() {
        assert_eq!(
            compile_and_run(
                "defmodule Demo do\n  def run do\n    [1, 2, 3]\n  end\nend"
            ),
            Ok(RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ]))
        );
    }
}
