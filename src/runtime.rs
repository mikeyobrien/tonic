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
                    evaluate_ops(program, after, &mut after_env, &mut after_stack)?;
                }

                if let Some(err) = final_err {
                    return Err(err);
                }

                if let Some(v) = early_return {
                    return Ok(Some(v));
                }
            }
            IrOp::Raise { offset } => {
                let value = pop_value(stack, *offset, "raise")?;
                return Err(RuntimeError::raised(value, *offset));
            }
            IrOp::Cond { branches, offset } => {
                let mut matched = false;
                for branch in branches {
                    let mut cond_env = env.clone();
                    let mut cond_stack = Vec::new();
                    evaluate_ops(program, &branch.condition_ops, &mut cond_env, &mut cond_stack)?;
                    let condition_value = pop_value(&mut cond_stack, *offset, "cond condition")?;
                    let truthy =
                        !matches!(condition_value, RuntimeValue::Nil | RuntimeValue::Bool(false));
                    if truthy {
                        matched = true;
                        if let Some(ret) =
                            evaluate_ops(program, &branch.ops, &mut cond_env, stack)?
                        {
                            return Ok(Some(ret));
                        }
                        break;
                    }
                }
                if !matched {
                    return Err(RuntimeError::at_offset(
                        "no cond clause was satisfied",
                        *offset,
                    ));
                }
            }
            IrOp::With {
                clauses,
                body_ops,
                else_branches,
                offset,
            } => {
                let mut with_env = env.clone();
                let mut matched = true;
                let mut failed_value = None;

                for clause in clauses {
                    let mut clause_stack = Vec::new();
                    evaluate_ops(program, &clause.ops, &mut with_env, &mut clause_stack)?;
                    let value = pop_value(&mut clause_stack, *offset, "with clause")?;

                    let mut bindings = HashMap::new();
                    if match_pattern(&value, &clause.pattern, &with_env, &mut bindings) {
                        for (k, v) in bindings {
                            with_env.insert(k, v);
                        }
                    } else {
                        matched = false;
                        failed_value = Some(value);
                        break;
                    }
                }

                if matched {
                    if let Some(ret) =
                        evaluate_ops(program, body_ops, &mut with_env, stack)?
                    {
                        return Ok(Some(ret));
                    }
                } else if let Some(failed) = failed_value {
                    let mut else_matched = false;
                    for branch in else_branches {
                        let mut bindings = HashMap::new();
                        if match_pattern(&failed, &branch.pattern, env, &mut bindings) {
                            let mut branch_env = env.clone();
                            for (k, v) in bindings {
                                branch_env.insert(k, v);
                            }
                            else_matched = true;
                            if let Some(ret) =
                                evaluate_ops(program, &branch.ops, &mut branch_env, stack)?
                            {
                                return Ok(Some(ret));
                            }
                            break;
                        }
                    }
                    if !else_matched {
                        return Err(RuntimeError::at_offset(
                            format!(
                                "no with else clause matching: {}",
                                failed.render()
                            ),
                            *offset,
                        ));
                    }
                }
            }
            IrOp::MakeTuple { offset } => {
                let right = pop_value(stack, *offset, "tuple right")?;
                let left = pop_value(stack, *offset, "tuple left")?;
                stack.push(RuntimeValue::Tuple(Box::new(left), Box::new(right)));
            }
            IrOp::MakeMap { size, offset } => {
                let mut entries = Vec::with_capacity(*size);
                for _ in 0..*size {
                    let value = pop_value(stack, *offset, "map value")?;
                    let key = pop_value(stack, *offset, "map key")?;
                    entries.push((key, value));
                }
                entries.reverse();
                stack.push(RuntimeValue::Map(entries));
            }
            IrOp::MakeKeyword { size, offset } => {
                let mut entries = Vec::with_capacity(*size);
                for _ in 0..*size {
                    let value = pop_value(stack, *offset, "keyword value")?;
                    let key = pop_value(stack, *offset, "keyword key")?;
                    entries.push((key, value));
                }
                entries.reverse();
                stack.push(RuntimeValue::Keyword(entries));
            }
            IrOp::MakeList { size, offset } => {
                let mut items = Vec::with_capacity(*size);
                for _ in 0..*size {
                    items.push(pop_value(stack, *offset, "list item")?);
                }
                items.reverse();
                stack.push(RuntimeValue::List(items));
            }
            IrOp::MakeResultOk { offset } => {
                let value = pop_value(stack, *offset, "ok")?;
                stack.push(RuntimeValue::ResultOk(Box::new(value)));
            }
            IrOp::MakeResultErr { offset } => {
                let value = pop_value(stack, *offset, "err")?;
                stack.push(RuntimeValue::ResultErr(Box::new(value)));
            }
            IrOp::For {
                generator,
                body_ops,
                into_ops,
                reduce_ops,
                offset,
            } => {
                let result = evaluate_for(
                    program,
                    generator,
                    body_ops,
                    into_ops.as_deref(),
                    reduce_ops.as_deref(),
                    env,
                    *offset,
                )?;
                stack.push(result);
            }
            IrOp::Pipe { ops, offset } => {
                let left = pop_value(stack, *offset, "pipe")?;
                let mut pipe_stack = vec![left];
                evaluate_ops(program, ops, env, &mut pipe_stack)?;
                if let Some(v) = pipe_stack.pop() {
                    stack.push(v);
                }
            }
            IrOp::UpdateMap { updates, offset } => {
                let map_value = pop_value(stack, *offset, "map update")?;
                match map_value {
                    RuntimeValue::Map(mut entries) => {
                        for (key_ops, value_ops) in updates {
                            let mut key_stack = Vec::new();
                            evaluate_ops(program, key_ops, env, &mut key_stack)?;
                            let key = pop_value(&mut key_stack, *offset, "map update key")?;

                            let mut value_stack = Vec::new();
                            evaluate_ops(program, value_ops, env, &mut value_stack)?;
                            let value =
                                pop_value(&mut value_stack, *offset, "map update value")?;

                            if let Some(existing) =
                                entries.iter_mut().find(|(k, _)| k == &key)
                            {
                                existing.1 = value;
                            } else {
                                return Err(RuntimeError::at_offset(
                                    format!(
                                        "key not found in map update: {}",
                                        key.render()
                                    ),
                                    *offset,
                                ));
                            }
                        }
                        stack.push(RuntimeValue::Map(entries));
                    }
                    other => {
                        return Err(RuntimeError::at_offset(
                            format!(
                                "map update requires map, found {}",
                                other.kind_label()
                            ),
                            *offset,
                        ));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn evaluate_for(
    program: &IrProgram,
    generator: &IrForGenerator,
    body_ops: &[IrOp],
    into_ops: Option<&[IrOp]>,
    reduce_ops: Option<&[IrOp]>,
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let items = collect_for_items(program, generator, env, offset)?;

    if let Some(reduce_ops) = reduce_ops {
        evaluate_for_reduce(program, &items, body_ops, reduce_ops, env, offset)
    } else {
        evaluate_for_collect(program, &items, body_ops, into_ops, env, offset)
    }
}

fn collect_for_items(
    program: &IrProgram,
    generator: &IrForGenerator,
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<Vec<(RuntimeValue, HashMap<String, RuntimeValue>)>, RuntimeError> {
    match generator {
        IrForGenerator::Single { source_ops, pattern, filter_ops } => {
            let mut source_stack = Vec::new();
            evaluate_ops(program, source_ops, env, &mut source_stack)?;
            let source = pop_value(&mut source_stack, offset, "for source")?;

            let items_iter: Vec<RuntimeValue> = match source {
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
                    let mut current = start;
                    if step > 0 {
                        while current <= end {
                            items.push(RuntimeValue::Int(current));
                            current += step;
                        }
                    } else if step < 0 {
                        while current >= end {
                            items.push(RuntimeValue::Int(current));
                            current += step;
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
                        format!("for requires iterable, found {}", other.kind_label()),
                        offset,
                    ));
                }
            };

            let mut result = Vec::new();
            for item in items_iter {
                let mut bindings = HashMap::new();
                if !match_pattern(&item, pattern, env, &mut bindings) {
                    continue;
                }

                let mut item_env = env.clone();
                for (k, v) in bindings {
                    item_env.insert(k, v);
                }

                if let Some(filter_ops) = filter_ops {
                    let mut filter_stack = Vec::new();
                    evaluate_ops(program, filter_ops, &mut item_env, &mut filter_stack)?;
                    let filter_val = pop_value(&mut filter_stack, offset, "for filter")?;
                    if matches!(filter_val, RuntimeValue::Nil | RuntimeValue::Bool(false)) {
                        continue;
                    }
                }

                result.push((item, item_env));
            }
            Ok(result)
        }
        IrForGenerator::Nested { generators } => {
            collect_nested_for_items(program, generators, env, offset)
        }
    }
}

fn collect_nested_for_items(
    program: &IrProgram,
    generators: &[crate::ir::IrForSingleGenerator],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<Vec<(RuntimeValue, HashMap<String, RuntimeValue>)>, RuntimeError> {
    if generators.is_empty() {
        return Ok(vec![(RuntimeValue::Nil, env.clone())]);
    }

    let first = &generators[0];
    let rest = &generators[1..];

    let mut source_stack = Vec::new();
    evaluate_ops(program, &first.source_ops, env, &mut source_stack)?;
    let source = pop_value(&mut source_stack, offset, "for source")?;

    let items_iter: Vec<RuntimeValue> = match source {
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
            let mut current = start;
            if step > 0 {
                while current <= end {
                    items.push(RuntimeValue::Int(current));
                    current += step;
                }
            } else if step < 0 {
                while current >= end {
                    items.push(RuntimeValue::Int(current));
                    current += step;
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
                format!("for requires iterable, found {}", other.kind_label()),
                offset,
            ));
        }
    };

    let mut result = Vec::new();
    for item in items_iter {
        let mut bindings = HashMap::new();
        if !match_pattern(&item, &first.pattern, env, &mut bindings) {
            continue;
        }

        let mut item_env = env.clone();
        for (k, v) in bindings {
            item_env.insert(k, v);
        }

        if let Some(filter_ops) = &first.filter_ops {
            let mut filter_stack = Vec::new();
            evaluate_ops(program, filter_ops, &mut item_env, &mut filter_stack)?;
            let filter_val = pop_value(&mut filter_stack, offset, "for filter")?;
            if matches!(filter_val, RuntimeValue::Nil | RuntimeValue::Bool(false)) {
                continue;
            }
        }

        let nested_results = collect_nested_for_items(program, rest, &mut item_env, offset)?;
        for (nested_item, nested_env) in nested_results {
            if matches!(nested_item, RuntimeValue::Nil) && rest.is_empty() {
                result.push((item.clone(), item_env.clone()));
            } else {
                result.push((nested_item, nested_env));
            }
        }
    }
    Ok(result)
}

fn evaluate_for_collect(
    program: &IrProgram,
    items: &[(RuntimeValue, HashMap<String, RuntimeValue>)],
    body_ops: &[IrOp],
    into_ops: Option<&[IrOp]>,
    env: &mut HashMap<String, RuntimeValue>,
    _offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let mut results = Vec::new();

    for (_item, item_env) in items {
        let mut body_env = item_env.clone();
        let mut body_stack = Vec::new();

        match evaluate_ops(program, body_ops, &mut body_env, &mut body_stack) {
            Ok(Some(_ret)) => {
                // Early return from body - skip this item
            }
            Ok(None) => {
                if let Some(v) = body_stack.pop() {
                    results.push(v);
                }
            }
            Err(e) => return Err(e),
        }
    }

    if let Some(into_ops) = into_ops {
        let mut into_env = env.clone();
        let result_list = RuntimeValue::List(results);
        into_env.insert("__for_results".to_string(), result_list);
        let mut into_stack = Vec::new();
        evaluate_ops(program, into_ops, &mut into_env, &mut into_stack)?;
        Ok(into_stack.pop().unwrap_or(RuntimeValue::Nil))
    } else {
        Ok(RuntimeValue::List(results))
    }
}

fn evaluate_for_reduce(
    program: &IrProgram,
    items: &[(RuntimeValue, HashMap<String, RuntimeValue>)],
    body_ops: &[IrOp],
    reduce_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
    _offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    // First evaluate the initial accumulator
    let mut acc_stack = Vec::new();
    evaluate_ops(program, reduce_ops, env, &mut acc_stack)?;
    let mut acc = acc_stack.pop().unwrap_or(RuntimeValue::Nil);

    for (_item, item_env) in items {
        let mut body_env = item_env.clone();
        body_env.insert(FOR_REDUCE_ACC_BINDING.to_string(), acc);
        let mut body_stack = Vec::new();

        match evaluate_ops(program, body_ops, &mut body_env, &mut body_stack) {
            Ok(Some(ret)) => {
                acc = ret;
            }
            Ok(None) => {
                acc = body_stack.pop().unwrap_or(RuntimeValue::Nil);
            }
            Err(e) => return Err(e),
        }
    }

    Ok(acc)
}

fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();

    match callee {
        IrCallTarget::Named(name) => evaluate_function(program, name, &args, offset),
        IrCallTarget::NativeOp(op) => {
            native_runtime::call_native_op(op, &args, offset).map_err(map_native_runtime_error)
        }
    }
}

fn evaluate_call_value(
    program: &IrProgram,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();
    let callee = stack.pop().ok_or_else(|| RuntimeError::at_offset("empty stack", offset))?;

    match callee {
        RuntimeValue::Closure(closure) => {
            if closure.params.len() != args.len() {
                return Err(RuntimeError::at_offset(
                    format!(
                        "closure arity mismatch: expected {} args, found {}",
                        closure.params.len(),
                        args.len()
                    ),
                    offset,
                ));
            }

            let mut closure_env = closure.env.clone();
            for (param, arg) in closure.params.iter().zip(args.iter()) {
                closure_env.insert(param.clone(), arg.clone());
            }

            let mut closure_stack = Vec::new();
            if let Some(ret) = evaluate_ops(program, &closure.ops, &mut closure_env, &mut closure_stack)? {
                return Ok(ret);
            }

            closure_stack.pop().ok_or_else(|| RuntimeError::at_offset("closure returned no value", offset))
        }
        other => Err(RuntimeError::at_offset(
            format!("call value requires function, found {}", other.kind_label()),
            offset,
        )),
    }
}

fn evaluate_guard_ops(
    program: &IrProgram,
    guard_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
) -> Result<bool, RuntimeError> {
    let mut stack = Vec::new();
    evaluate_ops(program, guard_ops, env, &mut stack)?;
    Ok(matches!(stack.last(), Some(RuntimeValue::Bool(true))))
}

fn pop_value(
    stack: &mut Vec<RuntimeValue>,
    offset: usize,
    context: &str,
) -> Result<RuntimeValue, RuntimeError> {
    stack.pop().ok_or_else(|| {
        RuntimeError::at_offset(
            format!("empty stack in {context}"),
            offset,
        )
    })
}

fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    match pattern {
        IrPattern::Wildcard => true,
        IrPattern::Bind(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        IrPattern::Pin(name) => {
            let pinned = env.get(name).or_else(|| bindings.get(name));
            pinned.map(|v| v == value).unwrap_or(false)
        }
        IrPattern::Int(expected) => matches!(value, RuntimeValue::Int(v) if v == expected),
        IrPattern::Float(expected) => matches!(value, RuntimeValue::Float(v) if v == expected),
        IrPattern::Bool(expected) => matches!(value, RuntimeValue::Bool(v) if v == expected),
        IrPattern::Nil => matches!(value, RuntimeValue::Nil),
        IrPattern::String(expected) => matches!(value, RuntimeValue::String(v) if v == expected),
        IrPattern::Atom(expected) => matches!(value, RuntimeValue::Atom(v) if v == expected),
        IrPattern::Tuple(left_pattern, right_pattern) => match value {
            RuntimeValue::Tuple(left, right) => {
                match_pattern(left, left_pattern, env, bindings)
                    && match_pattern(right, right_pattern, env, bindings)
            }
            _ => false,
        },
        IrPattern::List(patterns) => match value {
            RuntimeValue::List(items) => {
                if patterns.len() != items.len() {
                    return false;
                }
                patterns
                    .iter()
                    .zip(items.iter())
                    .all(|(p, v)| match_pattern(v, p, env, bindings))
            }
            _ => false,
        },
        IrPattern::ListHeadTail(head_pattern, tail_pattern) => match value {
            RuntimeValue::List(items) => {
                if items.is_empty() {
                    return false;
                }
                let head = &items[0];
                let tail = RuntimeValue::List(items[1..].to_vec());
                match_pattern(head, head_pattern, env, bindings)
                    && match_pattern(&tail, tail_pattern, env, bindings)
            }
            _ => false,
        },
        IrPattern::Map(pattern_entries) => match value {
            RuntimeValue::Map(entries) => pattern_entries.iter().all(|(key_pattern, val_pattern)| {
                entries.iter().any(|(k, v)| {
                    let mut key_bindings = HashMap::new();
                    if match_pattern(k, key_pattern, env, &mut key_bindings) {
                        let mut val_bindings = HashMap::new();
                        if match_pattern(v, val_pattern, env, &mut val_bindings) {
                            bindings.extend(key_bindings);
                            bindings.extend(val_bindings);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
            }),
            _ => false,
        },
        IrPattern::ResultOk(inner) => match value {
            RuntimeValue::ResultOk(inner_value) => match_pattern(inner_value, inner, env, bindings),
            _ => false,
        },
        IrPattern::ResultErr(inner) => match value {
            RuntimeValue::ResultErr(inner_value) => {
                match_pattern(inner_value, inner, env, bindings)
            }
            _ => false,
        },
    }
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
        IrOp::Return { offset } => *offset,
        IrOp::Question { offset } => *offset,
        IrOp::Case { offset, .. } => *offset,
        IrOp::Try { offset, .. } => *offset,
        IrOp::Raise { offset } => *offset,
        IrOp::Cond { offset, .. } => *offset,
        IrOp::With { offset, .. } => *offset,
        IrOp::MakeTuple { offset } => *offset,
        IrOp::MakeMap { offset, .. } => *offset,
        IrOp::MakeKeyword { offset, .. } => *offset,
        IrOp::MakeList { offset, .. } => *offset,
        IrOp::MakeResultOk { offset } => *offset,
        IrOp::MakeResultErr { offset } => *offset,
        IrOp::For { offset, .. } => *offset,
        IrOp::Pipe { offset, .. } => *offset,
        IrOp::UpdateMap { offset, .. } => *offset,
        IrOp::ToString { offset } => *offset,
    }
}

fn map_native_runtime_error(err: native_runtime::NativeRuntimeError) -> RuntimeError {
    RuntimeError {
        message: err.message,
        offset: Some(err.offset),
        raised_value: err.raised_value.map(map_native_value),
    }
}

fn map_native_value(value: native_runtime::NativeRuntimeValue) -> RuntimeValue {
    match value {
        native_runtime::NativeRuntimeValue::Int(i) => RuntimeValue::Int(i),
        native_runtime::NativeRuntimeValue::Float(f) => RuntimeValue::Float(f),
        native_runtime::NativeRuntimeValue::Bool(b) => RuntimeValue::Bool(b),
        native_runtime::NativeRuntimeValue::Nil => RuntimeValue::Nil,
        native_runtime::NativeRuntimeValue::String(s) => RuntimeValue::String(s),
        native_runtime::NativeRuntimeValue::Atom(a) => RuntimeValue::Atom(a),
        native_runtime::NativeRuntimeValue::ResultOk(v) => {
            RuntimeValue::ResultOk(Box::new(map_native_value(*v)))
        }
        native_runtime::NativeRuntimeValue::ResultErr(v) => {
            RuntimeValue::ResultErr(Box::new(map_native_value(*v)))
        }
        native_runtime::NativeRuntimeValue::Tuple(l, r) => RuntimeValue::Tuple(
            Box::new(map_native_value(*l)),
            Box::new(map_native_value(*r)),
        ),
        native_runtime::NativeRuntimeValue::Map(entries) => RuntimeValue::Map(
            entries
                .into_iter()
                .map(|(k, v)| (map_native_value(k), map_native_value(v)))
                .collect(),
        ),
        native_runtime::NativeRuntimeValue::Keyword(entries) => RuntimeValue::Keyword(
            entries
                .into_iter()
                .map(|(k, v)| (map_native_value(k), map_native_value(v)))
                .collect(),
        ),
        native_runtime::NativeRuntimeValue::List(items) => {
            RuntimeValue::List(items.into_iter().map(map_native_value).collect())
        }
        native_runtime::NativeRuntimeValue::Range(s, e) => RuntimeValue::Range(s, e),
        native_runtime::NativeRuntimeValue::SteppedRange(s, e, step) => {
            RuntimeValue::SteppedRange(s, e, step)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        IrCaseBranch, IrCondBranch, IrForGenerator, IrForSingleGenerator, IrFunction,
        IrRescueBranch, IrTryCatchBranch, IrWithClause, IrWithElseBranch,
    };

    fn make_program(functions: Vec<IrFunction>) -> IrProgram {
        IrProgram { functions }
    }

    #[test]
    fn test_simple_return() {
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
        assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(42)));
    }

    #[test]
    fn test_add() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 1, offset: 0 },
                IrOp::ConstInt { value: 2, offset: 0 },
                IrOp::AddInt { offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(3)));
    }

    #[test]
    fn test_missing_function() {
        let program = make_program(vec![]);
        assert!(evaluate_entrypoint(&program).is_err());
    }

    #[test]
    fn test_function_call() {
        let program = make_program(vec![
            IrFunction {
                name: "Demo.run".to_string(),
                params: vec![],
                param_patterns: None,
                guard_ops: None,
                ops: vec![
                    IrOp::Call {
                        callee: IrCallTarget::Named("Demo.helper".to_string()),
                        argc: 0,
                        offset: 0,
                    },
                    IrOp::Return { offset: 0 },
                ],
            },
            IrFunction {
                name: "Demo.helper".to_string(),
                params: vec![],
                param_patterns: None,
                guard_ops: None,
                ops: vec![
                    IrOp::ConstInt { value: 99, offset: 0 },
                    IrOp::Return { offset: 0 },
                ],
            },
        ]);
        assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(99)));
    }

    #[test]
    fn test_load_variable() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec!["x".to_string()],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(
            evaluate_function(&program, "Demo.run", &[RuntimeValue::Int(7)], 0),
            Ok(RuntimeValue::Int(7))
        );
    }

    #[test]
    fn test_case_basic() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 1, offset: 0 },
                IrOp::Case {
                    branches: vec![
                        IrCaseBranch {
                            pattern: IrPattern::Int(1),
                            guard_ops: None,
                            ops: vec![
                                IrOp::ConstAtom { value: "one".to_string(), offset: 0 },
                                IrOp::Return { offset: 0 },
                            ],
                        },
                        IrCaseBranch {
                            pattern: IrPattern::Wildcard,
                            guard_ops: None,
                            ops: vec![
                                IrOp::ConstAtom { value: "other".to_string(), offset: 0 },
                                IrOp::Return { offset: 0 },
                            ],
                        },
                    ],
                    offset: 0,
                },
            ],
        }]);
        assert_eq!(
            evaluate_entrypoint(&program),
            Ok(RuntimeValue::Atom("one".to_string()))
        );
    }

    #[test]
    fn test_match_op() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 42, offset: 0 },
                IrOp::Match { pattern: IrPattern::Bind("x".to_string()), offset: 0 },
                IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(42)));
    }

    #[test]
    fn test_for_collect() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::For {
                    generator: IrForGenerator::Single {
                        source_ops: vec![
                            IrOp::ConstInt { value: 1, offset: 0 },
                            IrOp::ConstInt { value: 3, offset: 0 },
                            IrOp::Range { offset: 0 },
                        ],
                        pattern: IrPattern::Bind("x".to_string()),
                        filter_ops: None,
                    },
                    body_ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::ConstInt { value: 2, offset: 0 },
                        IrOp::MulInt { offset: 0 },
                    ],
                    into_ops: None,
                    reduce_ops: None,
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(
            evaluate_entrypoint(&program),
            Ok(RuntimeValue::List(vec![
                RuntimeValue::Int(2),
                RuntimeValue::Int(4),
                RuntimeValue::Int(6),
            ]))
        );
    }

    #[test]
    fn test_cond() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::Cond {
                    branches: vec![
                        IrCondBranch {
                            condition_ops: vec![IrOp::ConstBool { value: false, offset: 0 }],
                            ops: vec![
                                IrOp::ConstAtom { value: "no".to_string(), offset: 0 },
                                IrOp::Return { offset: 0 },
                            ],
                        },
                        IrCondBranch {
                            condition_ops: vec![IrOp::ConstBool { value: true, offset: 0 }],
                            ops: vec![
                                IrOp::ConstAtom { value: "yes".to_string(), offset: 0 },
                                IrOp::Return { offset: 0 },
                            ],
                        },
                    ],
                    offset: 0,
                },
            ],
        }]);
        assert_eq!(
            evaluate_entrypoint(&program),
            Ok(RuntimeValue::Atom("yes".to_string()))
        );
    }

    #[test]
    fn test_try_rescue() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::Try {
                    body_ops: vec![
                        IrOp::ConstString { value: "boom".to_string(), offset: 0 },
                        IrOp::Raise { offset: 0 },
                    ],
                    rescue_branches: vec![IrRescueBranch {
                        pattern: IrPattern::Bind("e".to_string()),
                        guard_ops: None,
                        ops: vec![
                            IrOp::ConstAtom { value: "rescued".to_string(), offset: 0 },
                            IrOp::Return { offset: 0 },
                        ],
                    }],
                    catch_branches: vec![],
                    after_ops: None,
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(
            evaluate_entrypoint(&program),
            Ok(RuntimeValue::Atom("rescued".to_string()))
        );
    }

    #[test]
    fn test_closure() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::MakeClosure {
                    params: vec!["x".to_string()],
                    ops: vec![
                        IrOp::LoadVariable { name: "x".to_string(), offset: 0 },
                        IrOp::ConstInt { value: 1, offset: 0 },
                        IrOp::AddInt { offset: 0 },
                        IrOp::Return { offset: 0 },
                    ],
                    offset: 0,
                },
                IrOp::ConstInt { value: 5, offset: 0 },
                IrOp::CallValue { argc: 1, offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);
        assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(6)));
    }

    #[test]
    fn test_make_list() {
        let program = make_program(vec![IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt { value: 1, offset: 0 },
                IrOp::ConstInt { value: 2, offset: 0 },
                IrOp::ConstInt { value: 3, offset: 0 },
                IrOp::MakeList { size: 3, offset: 0 },
                IrOp::Return { offset: 0 },
            ],
        }]);
        let mut result_stack = Vec::new();
        let program_ref = &program;
        let mut env = HashMap::new();
        evaluate_ops(
            program_ref,
            &program.functions[0].ops,
            &mut env,
            &mut result_stack,
        )
        .unwrap();
        assert_eq!(
            result_stack,
            vec![RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ])]
        );
    }
}
