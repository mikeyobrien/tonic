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
    Bitstring(Vec<u8>),
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
            Self::Bitstring(bytes) => {
                let hex: Vec<String> = bytes.iter().map(|b| format!("{b}")).collect();
                format!("<<{}>>", hex.join(", "))
            }
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
            Self::Bitstring(_) => "bitstring",
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
                    evaluate_ops(program, after, &mut after_env, &mut after_stack)?;
                }

                if let Some(ret) = early_return {
                    return Ok(Some(ret));
                }

                if let Some(err) = final_err {
                    return Err(err);
                }
            }
            IrOp::For {
                generators,
                into_ops,
                reduce_ops,
                body_ops,
                offset,
            } => {
                let result = evaluate_for(
                    program,
                    generators,
                    into_ops.as_deref(),
                    reduce_ops.as_deref(),
                    body_ops,
                    env,
                    *offset,
                )?;
                stack.push(result);
            }
            IrOp::Raise { offset } => {
                let value = pop_value(stack, *offset, "raise")?;
                return Err(RuntimeError::raised(value, *offset));
            }
            IrOp::Bitstring { count, offset } => {
                let mut bytes = Vec::with_capacity(*count);
                let mut elems = Vec::with_capacity(*count);
                for _ in 0..*count {
                    elems.push(pop_value(stack, *offset, "bitstring")?);
                }
                elems.reverse();
                for elem in elems {
                    match elem {
                        RuntimeValue::Int(i) if i >= 0 && i <= 255 => bytes.push(i as u8),
                        other => {
                            return Err(RuntimeError::at_offset(
                                format!("bitstring element must be a byte (0-255), got {}", other.render()),
                                *offset,
                            ));
                        }
                    }
                }
                stack.push(RuntimeValue::Bitstring(bytes));
            }
        }
    }

    Ok(None)
}

fn evaluate_for(
    program: &IrProgram,
    generators: &[IrForGenerator],
    into_ops: Option<&[IrOp]>,
    reduce_ops: Option<&[IrOp]>,
    body_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    if generators.is_empty() {
        return Err(RuntimeError::at_offset(
            "for generator requires at least one generator",
            offset,
        ));
    }

    if let Some(reduce) = reduce_ops {
        return evaluate_for_reduce(
            program, generators, reduce, body_ops, env, offset,
        );
    }

    // Determine collection type for `into`
    let collect_map = if let Some(into) = into_ops {
        let mut into_stack = Vec::new();
        evaluate_ops(program, into, env, &mut into_stack)?;
        let into_val = into_stack.pop().unwrap_or(RuntimeValue::List(vec![]));
        matches!(into_val, RuntimeValue::Map(_))
    } else {
        false
    };

    let mut results: Vec<RuntimeValue> = Vec::new();
    evaluate_for_generators(
        program,
        generators,
        body_ops,
        env,
        offset,
        &mut results,
    )?;

    if collect_map {
        // Collect into a map: body should produce {key, value} tuples
        let mut map_entries: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
        for item in results {
            match item {
                RuntimeValue::Tuple(k, v) => map_entries.push((*k, *v)),
                other => {
                    return Err(RuntimeError::at_offset(
                        format!(
                            "for/into %{{}} expects tuple body, got {}",
                            other.kind_label()
                        ),
                        offset,
                    ))
                }
            }
        }
        Ok(RuntimeValue::Map(map_entries))
    } else {
        Ok(RuntimeValue::List(results))
    }
}

fn evaluate_for_generators(
    program: &IrProgram,
    generators: &[IrForGenerator],
    body_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
    results: &mut Vec<RuntimeValue>,
) -> Result<(), RuntimeError> {
    if generators.is_empty() {
        let mut body_env = env.clone();
        let mut body_stack = Vec::new();
        evaluate_ops(program, body_ops, &mut body_env, &mut body_stack)?;
        if let Some(value) = body_stack.pop() {
            results.push(value);
        }
        return Ok(());
    }

    let generator = &generators[0];
    let rest = &generators[1..];

    let mut source_stack = Vec::new();
    let mut source_env = env.clone();
    evaluate_ops(
        program,
        &generator.source_ops,
        &mut source_env,
        &mut source_stack,
    )?;
    let source = source_stack.pop().unwrap_or(RuntimeValue::List(vec![]));

    let items: Vec<RuntimeValue> = match source {
        RuntimeValue::List(items) => items,
        RuntimeValue::Range(start, end) => {
            if start <= end {
                (start..=end).map(RuntimeValue::Int).collect()
            } else {
                Vec::new()
            }
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            let mut items = Vec::new();
            if step > 0 {
                let mut i = start;
                while i <= end {
                    items.push(RuntimeValue::Int(i));
                    i += step;
                }
            } else if step < 0 {
                let mut i = start;
                while i >= end {
                    items.push(RuntimeValue::Int(i));
                    i += step;
                }
            }
            items
        }
        RuntimeValue::Map(entries) => entries
            .into_iter()
            .map(|(k, v)| RuntimeValue::Tuple(Box::new(k), Box::new(v)))
            .collect(),
        other => {
            return Err(RuntimeError::at_offset(
                format!("for generator source must be enumerable, got {}", other.kind_label()),
                offset,
            ))
        }
    };

    for item in items {
        let mut bindings = HashMap::new();
        if !match_pattern(&item, &generator.pattern, env, &mut bindings) {
            continue;
        }

        if let Some(guard_ops) = &generator.guard_ops {
            let mut guard_env = env.clone();
            guard_env.extend(bindings.clone());
            let guard_passed = evaluate_guard_ops(program, guard_ops, &mut guard_env)?;
            if !guard_passed {
                continue;
            }
        }

        for (name, value) in &bindings {
            env.insert(name.clone(), value.clone());
        }

        evaluate_for_generators(program, rest, body_ops, env, offset, results)?;
    }

    Ok(())
}

fn evaluate_for_reduce(
    program: &IrProgram,
    generators: &[IrForGenerator],
    reduce_ops: &[IrOp],
    body_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let mut acc_stack = Vec::new();
    evaluate_ops(program, reduce_ops, env, &mut acc_stack)?;
    let mut acc = acc_stack.pop().unwrap_or(RuntimeValue::Nil);

    evaluate_for_reduce_generators(
        program, generators, body_ops, env, offset, &mut acc,
    )?;

    Ok(acc)
}

fn evaluate_for_reduce_generators(
    program: &IrProgram,
    generators: &[IrForGenerator],
    body_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
    acc: &mut RuntimeValue,
) -> Result<(), RuntimeError> {
    if generators.is_empty() {
        env.insert(FOR_REDUCE_ACC_BINDING.to_string(), acc.clone());
        let mut body_env = env.clone();
        let mut body_stack = Vec::new();
        evaluate_ops(program, body_ops, &mut body_env, &mut body_stack)?;
        if let Some(new_acc) = body_stack.pop() {
            *acc = new_acc;
        }
        return Ok(());
    }

    let generator = &generators[0];
    let rest = &generators[1..];

    let mut source_stack = Vec::new();
    let mut source_env = env.clone();
    evaluate_ops(
        program,
        &generator.source_ops,
        &mut source_env,
        &mut source_stack,
    )?;
    let source = source_stack.pop().unwrap_or(RuntimeValue::List(vec![]));

    let items: Vec<RuntimeValue> = match source {
        RuntimeValue::List(items) => items,
        RuntimeValue::Range(start, end) => {
            if start <= end {
                (start..=end).map(RuntimeValue::Int).collect()
            } else {
                Vec::new()
            }
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            let mut items = Vec::new();
            if step > 0 {
                let mut i = start;
                while i <= end {
                    items.push(RuntimeValue::Int(i));
                    i += step;
                }
            } else if step < 0 {
                let mut i = start;
                while i >= end {
                    items.push(RuntimeValue::Int(i));
                    i += step;
                }
            }
            items
        }
        RuntimeValue::Map(entries) => entries
            .into_iter()
            .map(|(k, v)| RuntimeValue::Tuple(Box::new(k), Box::new(v)))
            .collect(),
        other => {
            return Err(RuntimeError::at_offset(
                format!("for generator source must be enumerable, got {}", other.kind_label()),
                offset,
            ))
        }
    };

    for item in items {
        let mut bindings = HashMap::new();
        if !match_pattern(&item, &generator.pattern, env, &mut bindings) {
            continue;
        }

        if let Some(guard_ops) = &generator.guard_ops {
            let mut guard_env = env.clone();
            guard_env.extend(bindings.clone());
            let guard_passed = evaluate_guard_ops(program, guard_ops, &mut guard_env)?;
            if !guard_passed {
                continue;
            }
        }

        for (name, value) in &bindings {
            env.insert(name.clone(), value.clone());
        }

        evaluate_for_reduce_generators(program, rest, body_ops, env, offset, acc)?;
    }

    Ok(())
}

fn evaluate_guard_ops(
    program: &IrProgram,
    guard_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
) -> Result<bool, RuntimeError> {
    let mut guard_stack = Vec::new();
    evaluate_ops(program, guard_ops, env, &mut guard_stack)?;
    let result = guard_stack.pop();
    Ok(matches!(result, Some(RuntimeValue::Bool(true))))
}

fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_value(stack, offset, "call arg")?);
    }
    args.reverse();

    match callee {
        IrCallTarget::Builtin { name } => {
            native_runtime::builtins::call_builtin(program, name, args, offset)
                .map_err(map_native_runtime_error)
        }
        IrCallTarget::Function { name } => evaluate_function(program, name, &args, offset),
    }
}

fn evaluate_call_value(
    program: &IrProgram,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_value(stack, offset, "call_value arg")?);
    }
    args.reverse();

    let callee = pop_value(stack, offset, "call_value callee")?;

    match callee {
        RuntimeValue::Closure(closure) => {
            let mut closure_env = closure.env.clone();
            if closure.params.len() != args.len() {
                return Err(RuntimeError::at_offset(
                    format!(
                        "closure arity mismatch: expected {} args, got {}",
                        closure.params.len(),
                        args.len()
                    ),
                    offset,
                ));
            }
            for (param, arg) in closure.params.iter().zip(args.iter()) {
                closure_env.insert(param.clone(), arg.clone());
            }
            let mut closure_stack = Vec::new();
            if let Some(ret) =
                evaluate_ops(program, &closure.ops, &mut closure_env, &mut closure_stack)?
            {
                return Ok(ret);
            }
            closure_stack
                .pop()
                .ok_or_else(|| RuntimeError::at_offset("closure returned nothing", offset))
        }
        other => Err(RuntimeError::at_offset(
            format!("call_value: expected closure, got {}", other.kind_label()),
            offset,
        )),
    }
}

fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    match pattern {
        IrPattern::Wildcard => true,
        IrPattern::Bind { name } => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        IrPattern::Pin { name } => {
            let pinned = env.get(name).or_else(|| bindings.get(name));
            Some(value) == pinned
        }
        IrPattern::Atom { value: atom } => matches!(value, RuntimeValue::Atom(a) if a == atom),
        IrPattern::Integer { value: int } => matches!(value, RuntimeValue::Int(i) if i == int),
        IrPattern::Bool { value: b } => matches!(value, RuntimeValue::Bool(v) if v == b),
        IrPattern::Nil => matches!(value, RuntimeValue::Nil),
        IrPattern::String { value: s } => matches!(value, RuntimeValue::String(v) if v == s),
        IrPattern::Tuple { items } => {
            if items.len() != 2 {
                return false;
            }
            match value {
                RuntimeValue::Tuple(left, right) => {
                    match_pattern(left, &items[0], env, bindings)
                        && match_pattern(right, &items[1], env, bindings)
                }
                _ => false,
            }
        }
        IrPattern::List { items, tail } => {
            let RuntimeValue::List(list) = value else {
                return false;
            };

            if tail.is_none() && list.len() != items.len() {
                return false;
            }

            if tail.is_some() && list.len() < items.len() {
                return false;
            }

            for (pattern, item) in items.iter().zip(list.iter()) {
                if !match_pattern(item, pattern, env, bindings) {
                    return false;
                }
            }

            if let Some(tail_pattern) = tail {
                let tail_list = RuntimeValue::List(list[items.len()..].to_vec());
                if !match_pattern(&tail_list, tail_pattern, env, bindings) {
                    return false;
                }
            }

            true
        }
        IrPattern::Map { entries } => {
            let RuntimeValue::Map(map_entries) = value else {
                return false;
            };

            for entry in entries {
                let matched_value = map_entries.iter().find_map(|(k, v)| {
                    let mut tmp_bindings = HashMap::new();
                    if match_pattern(k, &entry.key, env, &mut tmp_bindings) {
                        Some(v)
                    } else {
                        None
                    }
                });

                let Some(matched_value) = matched_value else {
                    return false;
                };

                if !match_pattern(matched_value, &entry.value, env, bindings) {
                    return false;
                }
            }

            true
        }
        IrPattern::Bitstring { segments } => {
            let RuntimeValue::Bitstring(bytes) = value else {
                return false;
            };

            if segments.len() != bytes.len() {
                return false;
            }

            for (segment, &byte) in segments.iter().zip(bytes.iter()) {
                match segment {
                    crate::ir::IrBitstringSegment::Literal { value } => {
                        if byte != *value {
                            return false;
                        }
                    }
                    crate::ir::IrBitstringSegment::Bind { name } => {
                        bindings.insert(name.clone(), RuntimeValue::Int(byte as i64));
                    }
                    crate::ir::IrBitstringSegment::Wildcard => {}
                }
            }

            true
        }
    }
}

fn pop_value(
    stack: &mut Vec<RuntimeValue>,
    offset: usize,
    context: &str,
) -> Result<RuntimeValue, RuntimeError> {
    stack.pop().ok_or_else(|| {
        RuntimeError::at_offset(format!("stack underflow in {context}"), offset)
    })
}

fn ir_op_offset(op: &IrOp) -> usize {
    match op {
        IrOp::ConstInt { offset, .. }
        | IrOp::ConstFloat { offset, .. }
        | IrOp::ConstBool { offset, .. }
        | IrOp::ConstNil { offset }
        | IrOp::ConstString { offset, .. }
        | IrOp::ToString { offset }
        | IrOp::Call { offset, .. }
        | IrOp::MakeClosure { offset, .. }
        | IrOp::CallValue { offset, .. }
        | IrOp::Question { offset }
        | IrOp::Case { offset, .. }
        | IrOp::Try { offset, .. }
        | IrOp::Raise { offset }
        | IrOp::For { offset, .. }
        | IrOp::LoadVariable { offset, .. }
        | IrOp::ConstAtom { offset, .. }
        | IrOp::AddInt { offset }
        | IrOp::SubInt { offset }
        | IrOp::MulInt { offset }
        | IrOp::DivInt { offset }
        | IrOp::CmpInt { offset, .. }
        | IrOp::Not { offset }
        | IrOp::Bang { offset }
        | IrOp::AndAnd { offset, .. }
        | IrOp::OrOr { offset, .. }
        | IrOp::And { offset, .. }
        | IrOp::Or { offset, .. }
        | IrOp::Concat { offset }
        | IrOp::In { offset }
        | IrOp::PlusPlus { offset }
        | IrOp::MinusMinus { offset }
        | IrOp::Range { offset }
        | IrOp::NotIn { offset }
        | IrOp::BitwiseAnd { offset }
        | IrOp::BitwiseOr { offset }
        | IrOp::BitwiseXor { offset }
        | IrOp::BitwiseNot { offset }
        | IrOp::BitwiseShiftLeft { offset }
        | IrOp::BitwiseShiftRight { offset }
        | IrOp::SteppedRange { offset }
        | IrOp::Bitstring { offset, .. }
        | IrOp::Match { offset, .. }
        | IrOp::Return { offset } => *offset,
    }
}

fn map_native_runtime_error(err: native_runtime::NativeRuntimeError) -> RuntimeError {
    RuntimeError {
        message: err.message,
        offset: err.offset,
        raised_value: err.raised_value.map(map_native_value_to_runtime_value),
    }
}

fn map_native_value_to_runtime_value(native: native_runtime::NativeRuntimeValue) -> RuntimeValue {
    match native {
        native_runtime::NativeRuntimeValue::Int(i) => RuntimeValue::Int(i),
        native_runtime::NativeRuntimeValue::Float(f) => RuntimeValue::Float(f),
        native_runtime::NativeRuntimeValue::Bool(b) => RuntimeValue::Bool(b),
        native_runtime::NativeRuntimeValue::Nil => RuntimeValue::Nil,
        native_runtime::NativeRuntimeValue::String(s) => RuntimeValue::String(s),
        native_runtime::NativeRuntimeValue::Atom(a) => RuntimeValue::Atom(a),
        native_runtime::NativeRuntimeValue::ResultOk(v) => {
            RuntimeValue::ResultOk(Box::new(map_native_value_to_runtime_value(*v)))
        }
        native_runtime::NativeRuntimeValue::ResultErr(v) => {
            RuntimeValue::ResultErr(Box::new(map_native_value_to_runtime_value(*v)))
        }
        native_runtime::NativeRuntimeValue::Tuple(a, b) => RuntimeValue::Tuple(
            Box::new(map_native_value_to_runtime_value(*a)),
            Box::new(map_native_value_to_runtime_value(*b)),
        ),
        native_runtime::NativeRuntimeValue::Map(entries) => RuntimeValue::Map(
            entries
                .into_iter()
                .map(|(k, v)| {
                    (
                        map_native_value_to_runtime_value(k),
                        map_native_value_to_runtime_value(v),
                    )
                })
                .collect(),
        ),
        native_runtime::NativeRuntimeValue::Keyword(entries) => RuntimeValue::Keyword(
            entries
                .into_iter()
                .map(|(k, v)| {
                    (
                        map_native_value_to_runtime_value(k),
                        map_native_value_to_runtime_value(v),
                    )
                })
                .collect(),
        ),
        native_runtime::NativeRuntimeValue::List(items) => RuntimeValue::List(
            items
                .into_iter()
                .map(map_native_value_to_runtime_value)
                .collect(),
        ),
        native_runtime::NativeRuntimeValue::Range(start, end) => {
            RuntimeValue::Range(start, end)
        }
        native_runtime::NativeRuntimeValue::SteppedRange(start, end, step) => {
            RuntimeValue::SteppedRange(start, end, step)
        }
        native_runtime::NativeRuntimeValue::Closure(c) => RuntimeValue::Closure(Box::new(
            RuntimeClosure {
                params: c.params,
                ops: c.ops,
                env: c
                    .env
                    .into_iter()
                    .map(|(k, v)| (k, map_native_value_to_runtime_value(v)))
                    .collect(),
            },
        )),
        native_runtime::NativeRuntimeValue::Bitstring(bytes) => RuntimeValue::Bitstring(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        IrCaseBranch, IrForGenerator, IrFunction, IrOp, IrPattern, IrProgram,
    };

    fn make_program(functions: Vec<IrFunction>) -> IrProgram {
        IrProgram { functions }
    }

    #[test]
    fn test_evaluate_simple_return() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 42, offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Int(42));
    }

    #[test]
    fn test_evaluate_add() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 10, offset: 0 },
                IrOp::ConstInt { value: 32, offset: 0 },
                IrOp::AddInt { offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Int(42));
    }

    #[test]
    fn test_match_atom_pattern() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstAtom { value: "ok".to_string(), offset: 0 },
                IrOp::Match {
                    pattern: IrPattern::Atom { value: "ok".to_string() },
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Atom("ok".to_string()));
    }

    #[test]
    fn test_for_list_comprehension() {
        // for x <- [1, 2, 3], do: x * 2
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Call {
                                callee: IrCallTarget::Builtin { name: "list".to_string() },
                                argc: 3,
                                offset: 0,
                            },
                        ],
                        guard_ops: None,
                    }],
                    into_ops: None,
                    reduce_ops: None,
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::ConstInt { value: 2, offset: 0 },
                        IrOp::MulInt { offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(2),
                RuntimeValue::Int(4),
                RuntimeValue::Int(6),
            ])
        );
    }

    #[test]
    fn test_bitstring_construct() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 72, offset: 0 },
                IrOp::ConstInt { value: 101, offset: 0 },
                IrOp::ConstInt { value: 108, offset: 0 },
                IrOp::Bitstring { count: 3, offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Bitstring(vec![72, 101, 108]));
    }

    #[test]
    fn test_bitstring_pattern_match() {
        use crate::ir::{IrBitstringSegment};
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 65, offset: 0 },
                IrOp::ConstInt { value: 66, offset: 0 },
                IrOp::Bitstring { count: 2, offset: 0 },
                IrOp::Match {
                    pattern: IrPattern::Bitstring {
                        segments: vec![
                            IrBitstringSegment::Literal { value: 65 },
                            IrBitstringSegment::Bind { name: "b".to_string() },
                        ],
                    },
                    offset: 0,
                },
                IrOp::LoadVariable { name: "b".to_string(), offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Int(66));
    }

    #[test]
    fn test_for_list_comprehension_with_filter() {
        // for x <- [1, 2, 3], x > 1, do: x * 2  => [4, 6]
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Call {
                                callee: IrCallTarget::Builtin { name: "list".to_string() },
                                argc: 3,
                                offset: 0,
                            },
                        ],
                        guard_ops: Some(vec![
                            IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::CmpInt { kind: crate::ir::CmpKind::Gt, offset: 0 },
                        ]),
                    }],
                    into_ops: None,
                    reduce_ops: None,
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::ConstInt { value: 2, offset: 0 },
                        IrOp::MulInt { offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(4),
                RuntimeValue::Int(6),
            ])
        );
    }

    #[test]
    fn test_for_reduce() {
        // for x <- [1, 2, 3], reduce: 0, do: acc + x  => 6
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Call {
                                callee: IrCallTarget::Builtin { name: "list".to_string() },
                                argc: 3,
                                offset: 0,
                            },
                        ],
                        guard_ops: None,
                    }],
                    into_ops: None,
                    reduce_ops: Some(vec![
                        IrOp::ConstInt { value: 0, offset: 0 },
                    ]),
                    body_ops: vec![
                        IrOp::LoadVariable { name: "__tonic_for_acc".to_string(), offset: 0 },
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::AddInt { offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(result, RuntimeValue::Int(6));
    }

    #[test]
    fn test_stepped_range_for() {
        // for x <- 1..10//2, do: x  => [1, 3, 5, 7, 9]
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 10, offset: 0 },
                            IrOp::Range { offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::SteppedRange { offset: 0 },
                        ],
                        guard_ops: None,
                    }],
                    into_ops: None,
                    reduce_ops: None,
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(3),
                RuntimeValue::Int(5),
                RuntimeValue::Int(7),
                RuntimeValue::Int(9),
            ])
        );
    }

    #[test]
    fn test_for_into_map() {
        // for x <- [1, 2, 3], into: %{}, do: {x, x * 2}
        // => %{1 => 2, 2 => 4, 3 => 6}
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Call {
                                callee: IrCallTarget::Builtin { name: "list".to_string() },
                                argc: 3,
                                offset: 0,
                            },
                        ],
                        guard_ops: None,
                    }],
                    into_ops: Some(vec![
                        IrOp::Call {
                            callee: IrCallTarget::Builtin { name: "map_empty".to_string() },
                            argc: 0,
                            offset: 0,
                        },
                    ]),
                    reduce_ops: None,
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::ConstInt { value: 2, offset: 0 },
                        IrOp::MulInt { offset: 0 },
                        IrOp::Call {
                            callee: IrCallTarget::Builtin { name: "tuple".to_string() },
                            argc: 2,
                            offset: 0,
                        },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Map(vec![
                (RuntimeValue::Int(1), RuntimeValue::Int(2)),
                (RuntimeValue::Int(2), RuntimeValue::Int(4)),
                (RuntimeValue::Int(3), RuntimeValue::Int(6)),
            ])
        );
    }

    #[test]
    fn test_push_to_stack_for_list() {
        // Simple for comprehension that collects items into a list
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generators: vec![IrForGenerator {
                        pattern: IrPattern::Bind { name: "x".to_string() },
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 2, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Call {
                                callee: IrCallTarget::Builtin { name: "list".to_string() },
                                argc: 3,
                                offset: 0,
                            },
                        ],
                        guard_ops: None,
                    }],
                    into_ops: None,
                    reduce_ops: None,
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);

        let result = evaluate_entrypoint(&program).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ])
        );
    }
}
