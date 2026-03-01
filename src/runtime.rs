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
                    match evaluate_ops(program, after, &mut after_env, &mut after_stack) {
                        Ok(ret) => {
                            if let Some(v) = ret {
                                return Ok(Some(v));
                            }
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }

                if let Some(err) = final_err {
                    return Err(err);
                }

                if let Some(ret) = early_return {
                    return Ok(Some(ret));
                }
            }
            IrOp::Raise { offset } => {
                let error_val = pop_value(stack, *offset, "raise")?;
                return Err(RuntimeError::raised(error_val, *offset));
            }
            IrOp::For {
                generators,
                into_ops,
                reduce_ops,
                body_ops,
                offset,
            } => {
                enum ForCollector {
                    List(Vec<RuntimeValue>),
                    Map(Vec<(RuntimeValue, RuntimeValue)>),
                    Keyword(Vec<(RuntimeValue, RuntimeValue)>),
                    Reduce(RuntimeValue),
                }

                fn collect_for_value(
                    collector: &mut ForCollector,
                    value: RuntimeValue,
                    offset: usize,
                ) -> Result<(), RuntimeError> {
                    match collector {
                        ForCollector::List(values) => values.push(value),
                        ForCollector::Map(entries) => {
                            let RuntimeValue::Tuple(key, entry_value) = value else {
                                return Err(RuntimeError::at_offset(
                                    format!(
                                        "for into map expects tuple {{key, value}}, found {}",
                                        value.kind_label()
                                    ),
                                    offset,
                                ));
                            };

                            let key = *key;
                            let entry_value = *entry_value;
                            if let Some(existing) =
                                entries.iter_mut().find(|(entry_key, _)| *entry_key == key)
                            {
                                existing.1 = entry_value;
                            } else {
                                entries.push((key, entry_value));
                            }
                        }
                        ForCollector::Keyword(entries) => {
                            let RuntimeValue::Tuple(key, entry_value) = value else {
                                return Err(RuntimeError::at_offset(
                                    format!(
                                        "for into keyword expects tuple {{key, value}}, found {}",
                                        value.kind_label()
                                    ),
                                    offset,
                                ));
                            };

                            let key = *key;
                            if !matches!(key, RuntimeValue::Atom(_)) {
                                return Err(RuntimeError::at_offset(
                                    format!(
                                        "for into keyword expects atom key, found {}",
                                        key.kind_label()
                                    ),
                                    offset,
                                ));
                            }

                            entries.push((key, *entry_value));
                        }
                        ForCollector::Reduce(_) => {
                            return Err(RuntimeError::at_offset(
                                "for internal error: reduce collector cannot accept yielded values",
                                offset,
                            ));
                        }
                    }

                    Ok(())
                }

                fn evaluate_generators(
                    program: &IrProgram,
                    generators: &[IrForGenerator],
                    gen_idx: usize,
                    env: &mut HashMap<String, RuntimeValue>,
                    body_ops: &[IrOp],
                    offset: usize,
                    collector: &mut ForCollector,
                ) -> Result<Option<RuntimeValue>, RuntimeError> {
                    if gen_idx >= generators.len() {
                        match collector {
                            ForCollector::Reduce(accumulator) => {
                                let mut reduce_env = env.clone();
                                reduce_env.insert(
                                    FOR_REDUCE_ACC_BINDING.to_string(),
                                    accumulator.clone(),
                                );

                                let mut reduce_stack = Vec::new();
                                if let Some(ret) = evaluate_ops(
                                    program,
                                    body_ops,
                                    &mut reduce_env,
                                    &mut reduce_stack,
                                )? {
                                    return Ok(Some(ret));
                                }

                                let next_acc =
                                    pop_value(&mut reduce_stack, offset, "for reduce body")?;
                                *accumulator = next_acc;
                            }
                            _ => {
                                let mut iteration_stack = Vec::new();
                                if let Some(ret) =
                                    evaluate_ops(program, body_ops, env, &mut iteration_stack)?
                                {
                                    return Ok(Some(ret));
                                }
                                let body_value =
                                    pop_value(&mut iteration_stack, offset, "for body")?;
                                collect_for_value(collector, body_value, offset)?;
                            }
                        }

                        return Ok(None);
                    }

                    let generator = &generators[gen_idx];
                    let mut gen_stack = Vec::new();
                    if let Some(ret) =
                        evaluate_ops(program, &generator.source_ops, env, &mut gen_stack)?
                    {
                        return Ok(Some(ret));
                    }
                    let enumerable = pop_value(&mut gen_stack, offset, "for generator")?;
                    let values = match enumerable {
                        RuntimeValue::List(values) => values,
                        RuntimeValue::Range(start, end) => {
                            (start..=end).map(RuntimeValue::Int).collect()
                        }
                        RuntimeValue::SteppedRange(start, end, step) => {
                            let mut v = Vec::new();
                            if step > 0 {
                                let mut i = start;
                                while i <= end {
                                    v.push(RuntimeValue::Int(i));
                                    i += step;
                                }
                            } else if step < 0 {
                                let mut i = start;
                                while i >= end {
                                    v.push(RuntimeValue::Int(i));
                                    i += step;
                                }
                            }
                            v
                        }
                        other => {
                            return Err(RuntimeError::at_offset(
                                format!("for expects list generator, found {}", other.kind_label()),
                                offset,
                            ));
                        }
                    };

                    for value in values {
                        let mut bindings = HashMap::new();
                        if !match_pattern(&value, &generator.pattern, env, &mut bindings) {
                            continue;
                        }

                        let mut iteration_env = env.clone();
                        for (name, bound_value) in bindings {
                            iteration_env.insert(name, bound_value);
                        }

                        if let Some(guard_ops) = &generator.guard_ops {
                            if !evaluate_guard_ops(program, guard_ops, &mut iteration_env)? {
                                continue;
                            }
                        }

                        if let Some(ret) = evaluate_generators(
                            program,
                            generators,
                            gen_idx + 1,
                            &mut iteration_env,
                            body_ops,
                            offset,
                            collector,
                        )? {
                            return Ok(Some(ret));
                        }
                    }

                    Ok(None)
                }

                if into_ops.is_some() && reduce_ops.is_some() {
                    return Err(RuntimeError::at_offset(
                        "for options 'reduce' and 'into' cannot be combined",
                        *offset,
                    ));
                }

                let mut collector = if let Some(ops) = reduce_ops {
                    let mut reduce_stack = Vec::new();
                    if let Some(ret) = evaluate_ops(program, ops, env, &mut reduce_stack)? {
                        return Ok(Some(ret));
                    }
                    let reduce_value = pop_value(&mut reduce_stack, *offset, "reduce")?;
                    ForCollector::Reduce(reduce_value)
                } else if let Some(ops) = into_ops {
                    let mut into_stack = Vec::new();
                    if let Some(ret) = evaluate_ops(program, ops, env, &mut into_stack)? {
                        return Ok(Some(ret));
                    }
                    let into_val = pop_value(&mut into_stack, *offset, "into")?;
                    match into_val {
                        RuntimeValue::List(values) => ForCollector::List(values),
                        RuntimeValue::Map(entries) => ForCollector::Map(entries),
                        RuntimeValue::Keyword(entries) => ForCollector::Keyword(entries),
                        other => {
                            return Err(RuntimeError::at_offset(
                                format!(
                                    "for into destination must be a list, map, or keyword, found {}",
                                    other.kind_label()
                                ),
                                *offset,
                            ));
                        }
                    }
                } else {
                    ForCollector::List(Vec::new())
                };

                if let Some(ret) = evaluate_generators(
                    program,
                    generators,
                    0,
                    env,
                    body_ops,
                    *offset,
                    &mut collector,
                )? {
                    return Ok(Some(ret));
                }

                let result = match collector {
                    ForCollector::List(values) => RuntimeValue::List(values),
                    ForCollector::Map(entries) => RuntimeValue::Map(entries),
                    ForCollector::Keyword(entries) => RuntimeValue::Keyword(entries),
                    ForCollector::Reduce(value) => value,
                };

                stack.push(result);
            }
        }
    }

    Ok(None)
}

fn evaluate_guard_ops(
    program: &IrProgram,
    guard_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
) -> Result<bool, RuntimeError> {
    let mut guard_stack = Vec::new();

    if let Some(ret) = evaluate_ops(program, guard_ops, env, &mut guard_stack)? {
        guard_stack.push(ret);
    }

    let guard_offset = guard_ops.first().map(ir_op_offset).unwrap_or(0);
    let guard_value = pop_value(&mut guard_stack, guard_offset, "guard")?;

    match guard_value {
        RuntimeValue::Bool(flag) => Ok(flag),
        other => Err(RuntimeError::at_offset(
            format!(
                "guard expression must evaluate to bool, found {}",
                other.kind_label()
            ),
            guard_offset,
        )),
    }
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
        | IrOp::NotIn { offset }
        | IrOp::PlusPlus { offset }
        | IrOp::MinusMinus { offset }
        | IrOp::Range { offset }
        | IrOp::BitwiseAnd { offset }
        | IrOp::BitwiseOr { offset }
        | IrOp::BitwiseXor { offset }
        | IrOp::BitwiseNot { offset }
        | IrOp::BitwiseShiftLeft { offset }
        | IrOp::BitwiseShiftRight { offset }
        | IrOp::SteppedRange { offset }
        | IrOp::Match { offset, .. }
        | IrOp::Return { offset } => *offset,
    }
}

fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    native_runtime::pattern::match_pattern(value, pattern, env, bindings)
}

fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args_start = stack.len().checked_sub(argc).ok_or_else(|| {
        RuntimeError::at_offset(
            format!("runtime stack underflow for call with {argc} args"),
            offset,
        )
    })?;

    match callee {
        IrCallTarget::Builtin { name } => {
            let args = stack.split_off(args_start);
            evaluate_builtin_call(name, args, offset)
        }
        IrCallTarget::Function { name } => {
            let value = evaluate_function(program, name, &stack[args_start..], offset)?;
            stack.truncate(args_start);
            Ok(value)
        }
    }
}

fn evaluate_call_value(
    program: &IrProgram,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args_start = stack.len().checked_sub(argc).ok_or_else(|| {
        RuntimeError::at_offset(
            format!("runtime stack underflow for closure call with {argc} args"),
            offset,
        )
    })?;

    let args = stack.split_off(args_start);
    let callee = stack
        .pop()
        .ok_or_else(|| RuntimeError::at_offset("missing function value for invocation", offset))?;

    match callee {
        RuntimeValue::Closure(closure) => evaluate_closure(program, closure.as_ref(), args, offset),
        other => Err(RuntimeError::at_offset(
            format!(
                "attempted to call non-function value: {}",
                other.kind_label()
            ),
            offset,
        )),
    }
}

fn evaluate_closure(
    program: &IrProgram,
    closure: &RuntimeClosure,
    args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    if args.len() != closure.params.len() {
        return Err(RuntimeError::at_offset(
            format!(
                "arity mismatch for anonymous function: expected {} args, found {}",
                closure.params.len(),
                args.len()
            ),
            offset,
        ));
    }

    let mut env = closure.env.clone();
    for (name, value) in closure.params.iter().zip(args.into_iter()) {
        env.insert(name.clone(), value);
    }

    let mut closure_stack = Vec::new();
    if let Some(value) = evaluate_ops(program, &closure.ops, &mut env, &mut closure_stack)? {
        Ok(value)
    } else {
        Err(RuntimeError::at_offset(
            "anonymous function ended without return",
            offset,
        ))
    }
}

fn evaluate_builtin_call(
    name: &str,
    args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    native_runtime::evaluate_builtin_call(name, args, offset).map_err(map_native_runtime_error)
}

fn pop_value(
    stack: &mut Vec<RuntimeValue>,
    offset: usize,
    op_name: &str,
) -> Result<RuntimeValue, RuntimeError> {
    stack.pop().ok_or_else(|| {
        RuntimeError::at_offset(format!("runtime stack underflow for {op_name}"), offset)
    })
}

fn map_native_runtime_error(error: native_runtime::NativeRuntimeError) -> RuntimeError {
    RuntimeError::at_offset(error.message().to_string(), error.offset())
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_builtin_call, evaluate_entrypoint, evaluate_ops, RuntimeError, RuntimeValue,
    };
    use crate::ir::{lower_ast_to_ir, IrCaseBranch, IrFunction, IrOp, IrPattern, IrProgram};
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;
    use std::collections::HashMap;

    #[test]
    fn evaluate_entrypoint_executes_integer_addition() {
        let source = "defmodule Demo do\n  def run() do\n    1 + 2\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize runtime fixture");
        let ast = parse_ast(&tokens).expect("parser should build runtime fixture ast");
        let ir = lower_ast_to_ir(&ast).expect("lowering should support runtime fixture");

        let value = evaluate_entrypoint(&ir).expect("runtime should evaluate arithmetic fixture");

        assert_eq!(value, RuntimeValue::Int(3));
    }

    #[test]
    fn evaluate_entrypoint_errors_when_demo_run_missing() {
        let source = "defmodule Demo do\n  def helper() do\n    1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize runtime fixture");
        let ast = parse_ast(&tokens).expect("parser should build runtime fixture ast");
        let ir = lower_ast_to_ir(&ast).expect("lowering should support runtime fixture");

        let error = evaluate_entrypoint(&ir).expect_err("runtime should reject missing Demo.run");

        assert_eq!(
            error,
            RuntimeError {
                message: "missing runtime function: Demo.run".to_string(),
                offset: None,
                raised_value: None,
            }
        );
    }

    #[test]
    fn evaluate_entrypoint_propagates_err_results_through_question() {
        let source =
            "defmodule Demo do\n  def fail() do\n    err(7)\n  end\n\n  def run() do\n    fail()?\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize runtime fixture");
        let ast = parse_ast(&tokens).expect("parser should build runtime fixture ast");
        let ir = lower_ast_to_ir(&ast).expect("lowering should support runtime fixture");

        let value = evaluate_entrypoint(&ir).expect("runtime should evaluate result fixture");

        assert_eq!(
            value,
            RuntimeValue::ResultErr(Box::new(RuntimeValue::Int(7)))
        );
    }

    #[test]
    fn evaluate_entrypoint_reports_deterministic_no_match_case_errors() {
        let ir = IrProgram {
            functions: vec![IrFunction {
                name: "Demo.run".to_string(),
                params: vec![],
                param_patterns: None,
                guard_ops: None,
                ops: vec![
                    IrOp::ConstInt {
                        value: 1,
                        offset: 37,
                    },
                    IrOp::Case {
                        branches: vec![IrCaseBranch {
                            pattern: IrPattern::Atom {
                                value: "ok".to_string(),
                            },
                            guard_ops: None,
                            ops: vec![IrOp::ConstInt {
                                value: 2,
                                offset: 55,
                            }],
                        }],
                        offset: 37,
                    },
                    IrOp::Return { offset: 37 },
                ],
            }],
        };

        let error =
            evaluate_entrypoint(&ir).expect_err("runtime should fail when no case branch matches");

        assert_eq!(error.to_string(), "no case clause matching at offset 37");
    }

    #[test]
    fn evaluate_builtin_collection_constructors_render_expected_shape() {
        let map = evaluate_builtin_call("map", vec![RuntimeValue::Int(1), RuntimeValue::Int(2)], 0)
            .expect("builtin map should produce a runtime map value");

        let keyword = evaluate_builtin_call(
            "keyword",
            vec![RuntimeValue::Int(3), RuntimeValue::Int(4)],
            0,
        )
        .expect("builtin keyword should produce a runtime keyword value");

        let tuple = evaluate_builtin_call("tuple", vec![map, keyword], 0)
            .expect("builtin tuple should produce a runtime tuple value");

        assert_eq!(tuple.render(), "{%{1 => 2}, [3: 4]}");
    }

    #[test]
    fn evaluate_builtin_protocol_dispatch_routes_tuple_and_map_values() {
        let tuple =
            evaluate_builtin_call("tuple", vec![RuntimeValue::Int(1), RuntimeValue::Int(2)], 0)
                .expect("builtin tuple should produce a runtime tuple value");
        let map = evaluate_builtin_call("map", vec![RuntimeValue::Int(3), RuntimeValue::Int(4)], 0)
            .expect("builtin map should produce a runtime map value");

        let tuple_impl = evaluate_builtin_call("protocol_dispatch", vec![tuple], 0)
            .expect("protocol dispatch should resolve tuple implementation");
        let map_impl = evaluate_builtin_call("protocol_dispatch", vec![map], 0)
            .expect("protocol dispatch should resolve map implementation");

        assert_eq!(tuple_impl, RuntimeValue::Int(1));
        assert_eq!(map_impl, RuntimeValue::Int(2));
    }

    #[test]
    fn evaluate_builtin_ok_moves_nested_payload_without_cloning() {
        let nested = RuntimeValue::ResultOk(Box::new(RuntimeValue::Int(5)));
        let original_inner_ptr = match &nested {
            RuntimeValue::ResultOk(inner) => inner.as_ref() as *const RuntimeValue as usize,
            _ => unreachable!("fixture should be nested result"),
        };

        let value =
            evaluate_builtin_call("ok", vec![nested], 0).expect("builtin ok should return result");

        let moved_inner_ptr = match value {
            RuntimeValue::ResultOk(outer) => match *outer {
                RuntimeValue::ResultOk(inner) => inner.as_ref() as *const RuntimeValue as usize,
                other => panic!("expected nested result payload, found {other:?}"),
            },
            other => panic!("expected ok result wrapper, found {other:?}"),
        };

        assert_eq!(moved_inner_ptr, original_inner_ptr);
    }

    #[test]
    fn evaluate_builtin_host_call_identity() {
        // Test calling the identity host function
        let result = evaluate_builtin_call(
            "host_call",
            vec![
                RuntimeValue::Atom("identity".to_string()),
                RuntimeValue::Int(42),
            ],
            0,
        );
        assert_eq!(result, Ok(RuntimeValue::Int(42)));
    }

    #[test]
    fn evaluate_builtin_host_call_sum_ints() {
        // Test calling the sum_ints host function
        let result = evaluate_builtin_call(
            "host_call",
            vec![
                RuntimeValue::Atom("sum_ints".to_string()),
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ],
            0,
        );
        assert_eq!(result, Ok(RuntimeValue::Int(6)));
    }

    #[test]
    fn evaluate_builtin_host_call_unknown_function() {
        // Test calling an unknown host function
        let result = evaluate_builtin_call(
            "host_call",
            vec![RuntimeValue::Atom("nonexistent".to_string())],
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn evaluate_builtin_host_call_requires_atom_key() {
        // Test that first argument must be an atom
        let result = evaluate_builtin_call(
            "host_call",
            vec![RuntimeValue::Int(42), RuntimeValue::Int(1)],
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn evaluate_builtin_host_call_requires_at_least_one_arg() {
        // Test that host_call requires at least the key argument
        let result = evaluate_builtin_call("host_call", vec![], 0);
        assert!(result.is_err());
    }

    #[test]
    fn evaluate_entrypoint_distinguishes_strict_not_and_relaxed_bang() {
        let source =
            "defmodule Demo do\n  def run() do\n    tuple(tuple(!nil, !1), not false)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize unary logical fixture");
        let ast = parse_ast(&tokens).expect("parser should build unary logical fixture ast");
        let ir = lower_ast_to_ir(&ast).expect("lowering should support unary logical fixture");

        let value =
            evaluate_entrypoint(&ir).expect("runtime should evaluate unary logical fixture");

        assert_eq!(value.render(), "{{true, false}, true}");
    }

    #[test]
    fn evaluate_entrypoint_rejects_not_for_non_boolean_values() {
        let source = "defmodule Demo do\n  def run() do\n    not 1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize strict-not fixture");
        let ast = parse_ast(&tokens).expect("parser should build strict-not fixture ast");
        let ir = lower_ast_to_ir(&ast).expect("lowering should support strict-not fixture");

        let error = evaluate_entrypoint(&ir)
            .expect_err("runtime should reject strict not on non-boolean values");

        assert_eq!(error.message, "badarg");
    }

    #[test]
    fn evaluate_ops_supports_list_membership_concat_and_subtract() {
        let program = IrProgram { functions: vec![] };
        let mut env = HashMap::new();

        let mut in_stack = vec![
            RuntimeValue::Int(2),
            RuntimeValue::List(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]),
        ];
        evaluate_ops(&program, &[IrOp::In { offset: 0 }], &mut env, &mut in_stack)
            .expect("runtime should evaluate list membership");
        assert_eq!(in_stack, vec![RuntimeValue::Bool(true)]);

        let mut plus_plus_stack = vec![
            RuntimeValue::List(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]),
            RuntimeValue::List(vec![RuntimeValue::Int(2), RuntimeValue::Int(3)]),
        ];
        evaluate_ops(
            &program,
            &[IrOp::PlusPlus { offset: 0 }],
            &mut env,
            &mut plus_plus_stack,
        )
        .expect("runtime should concatenate list values");
        assert_eq!(
            plus_plus_stack,
            vec![RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ])]
        );

        let mut minus_minus_stack = vec![
            RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ]),
            RuntimeValue::List(vec![RuntimeValue::Int(2), RuntimeValue::Int(4)]),
        ];
        evaluate_ops(
            &program,
            &[IrOp::MinusMinus { offset: 0 }],
            &mut env,
            &mut minus_minus_stack,
        )
        .expect("runtime should subtract list values deterministically");
        assert_eq!(
            minus_minus_stack,
            vec![RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ])]
        );
    }
}
