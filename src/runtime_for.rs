use super::*;

pub(super) fn evaluate_for(
    program: &IrProgram,
    generators: &[IrForGenerator],
    body_ops: &[IrOp],
    into_ops: Option<&[IrOp]>,
    reduce_ops: Option<&[IrOp]>,
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let items = collect_for_items(program, generators, env, offset)?;

    if let Some(reduce_ops) = reduce_ops {
        evaluate_for_reduce(program, &items, body_ops, reduce_ops, env, offset)
    } else {
        evaluate_for_collect(program, &items, body_ops, into_ops, env, offset)
    }
}

pub(super) fn iter_source_value(
    source: RuntimeValue,
    offset: usize,
) -> Result<Vec<RuntimeValue>, RuntimeError> {
    match source {
        RuntimeValue::List(items) => Ok(items),
        RuntimeValue::Range(start, end) => {
            if start <= end {
                Ok((start..=end).map(RuntimeValue::Int).collect())
            } else {
                Ok(Vec::new())
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
            Ok(items)
        }
        RuntimeValue::Map(entries) => Ok(entries
            .into_iter()
            .map(|(k, v)| RuntimeValue::Tuple(Box::new(k), Box::new(v)))
            .collect()),
        other => Err(RuntimeError::at_offset(
            format!("for requires iterable, found {}", other.kind_label()),
            offset,
        )),
    }
}

type ForItems = Vec<(RuntimeValue, HashMap<String, RuntimeValue>)>;

pub(super) fn collect_for_items(
    program: &IrProgram,
    generators: &[IrForGenerator],
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<ForItems, RuntimeError> {
    if generators.is_empty() {
        return Ok(vec![(RuntimeValue::Nil, env.clone())]);
    }

    let first = &generators[0];
    let rest = &generators[1..];

    let mut source_stack = Vec::new();
    evaluate_ops(program, &first.source_ops, env, &mut source_stack)?;
    let source = pop_value(&mut source_stack, offset, "for source")?;
    let items_iter = iter_source_value(source, offset)?;

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

        if let Some(guard_ops) = &first.guard_ops {
            let mut filter_stack = Vec::new();
            evaluate_ops(program, guard_ops, &mut item_env, &mut filter_stack)?;
            let filter_val = pop_value(&mut filter_stack, offset, "for guard")?;
            if matches!(filter_val, RuntimeValue::Nil | RuntimeValue::Bool(false)) {
                continue;
            }
        }

        if rest.is_empty() {
            result.push((item, item_env));
        } else {
            let nested = collect_for_items(program, rest, &mut item_env, offset)?;
            for (nested_item, nested_env) in nested {
                result.push((nested_item, nested_env));
            }
        }
    }
    Ok(result)
}

pub(super) fn evaluate_for_collect(
    program: &IrProgram,
    items: &[(RuntimeValue, HashMap<String, RuntimeValue>)],
    body_ops: &[IrOp],
    into_ops: Option<&[IrOp]>,
    env: &mut HashMap<String, RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    if let Some(into_ops) = into_ops {
        // Evaluate the seed expression to determine the destination type
        let mut seed_stack = Vec::new();
        evaluate_ops(program, into_ops, env, &mut seed_stack)?;
        let seed = seed_stack.pop().unwrap_or(RuntimeValue::Nil);

        match seed {
            RuntimeValue::Map(mut acc) => {
                for (_item, item_env) in items {
                    let mut body_env = item_env.clone();
                    let mut body_stack = Vec::new();
                    match evaluate_ops(program, body_ops, &mut body_env, &mut body_stack) {
                        Ok(Some(_)) => {}
                        Ok(None) => {
                            let v = body_stack.pop().unwrap_or(RuntimeValue::Nil);
                            match v {
                                RuntimeValue::Tuple(k, val) => acc.push((*k, *val)),
                                other => {
                                    return Err(RuntimeError::at_offset(
                                        format!(
                                            "for into map expects tuple {{key, value}}, found {}",
                                            other.kind_label()
                                        ),
                                        offset,
                                    ))
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(RuntimeValue::Map(acc))
            }
            RuntimeValue::Keyword(mut acc) => {
                for (_item, item_env) in items {
                    let mut body_env = item_env.clone();
                    let mut body_stack = Vec::new();
                    match evaluate_ops(program, body_ops, &mut body_env, &mut body_stack) {
                        Ok(Some(_)) => {}
                        Ok(None) => {
                            let v = body_stack.pop().unwrap_or(RuntimeValue::Nil);
                            match v {
                                RuntimeValue::Tuple(k, val) => acc.push((*k, *val)),
                                other => {
                                    return Err(RuntimeError::at_offset(
                                        format!(
                                        "for into keyword expects tuple {{key, value}}, found {}",
                                        other.kind_label()
                                    ),
                                        offset,
                                    ))
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(RuntimeValue::Keyword(acc))
            }
            RuntimeValue::List(mut acc) => {
                for (_item, item_env) in items {
                    let mut body_env = item_env.clone();
                    let mut body_stack = Vec::new();
                    match evaluate_ops(program, body_ops, &mut body_env, &mut body_stack) {
                        Ok(Some(_)) => {}
                        Ok(None) => {
                            if let Some(v) = body_stack.pop() {
                                acc.push(v);
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(RuntimeValue::List(acc))
            }
            other => Err(RuntimeError::at_offset(
                format!(
                    "for into destination must be a list, map, or keyword, found {}",
                    other.kind_label()
                ),
                offset,
            )),
        }
    } else {
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
        Ok(RuntimeValue::List(results))
    }
}

pub(super) fn evaluate_for_reduce(
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

pub(super) fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();

    match callee {
        IrCallTarget::Function { name } => evaluate_function(program, name, &args, offset),
        IrCallTarget::Builtin { name } => native_runtime::evaluate_builtin_call(name, args, offset)
            .map_err(map_native_runtime_error),
    }
}

pub(super) fn evaluate_call_value(
    program: &IrProgram,
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let args: Vec<RuntimeValue> = stack.drain(stack.len() - argc..).collect();
    let callee = stack
        .pop()
        .ok_or_else(|| RuntimeError::at_offset("empty stack", offset))?;

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
            if let Some(ret) =
                evaluate_ops(program, &closure.ops, &mut closure_env, &mut closure_stack)?
            {
                return Ok(ret);
            }

            closure_stack
                .pop()
                .ok_or_else(|| RuntimeError::at_offset("closure returned no value", offset))
        }
        other => Err(RuntimeError::at_offset(
            format!("call value requires function, found {}", other.kind_label()),
            offset,
        )),
    }
}

pub(super) fn evaluate_guard_ops(
    program: &IrProgram,
    guard_ops: &[IrOp],
    env: &mut HashMap<String, RuntimeValue>,
) -> Result<bool, RuntimeError> {
    let mut stack = Vec::new();
    evaluate_ops(program, guard_ops, env, &mut stack)?;
    Ok(matches!(stack.last(), Some(RuntimeValue::Bool(true))))
}

pub(super) fn pop_value(
    stack: &mut Vec<RuntimeValue>,
    offset: usize,
    context: &str,
) -> Result<RuntimeValue, RuntimeError> {
    stack
        .pop()
        .ok_or_else(|| RuntimeError::at_offset(format!("empty stack in {context}"), offset))
}

pub(super) fn match_pattern(
    value: &RuntimeValue,
    pattern: &IrPattern,
    env: &HashMap<String, RuntimeValue>,
    bindings: &mut HashMap<String, RuntimeValue>,
) -> bool {
    use crate::native_runtime::pattern::match_pattern as native_match;
    native_match(value, pattern, env, bindings)
}

pub(super) fn ir_op_offset(op: &IrOp) -> usize {
    match op {
        IrOp::ConstInt { offset, .. } => *offset,
        IrOp::ConstFloat { offset, .. } => *offset,
        IrOp::ConstBool { offset, .. } => *offset,
        IrOp::ConstNil { offset } => *offset,
        IrOp::ConstString { offset, .. } => *offset,
        IrOp::ConstAtom { offset, .. } => *offset,
        IrOp::ToString { offset } => *offset,
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
        IrOp::IntDiv { offset } => *offset,
        IrOp::RemInt { offset } => *offset,
        IrOp::CmpInt { offset, .. } => *offset,
        IrOp::Match { offset, .. } => *offset,
        IrOp::Return { offset } => *offset,
        IrOp::Question { offset } => *offset,
        IrOp::Case { offset, .. } => *offset,
        IrOp::Try { offset, .. } => *offset,
        IrOp::Raise { offset } => *offset,
        IrOp::For { offset, .. } => *offset,
        IrOp::Bitstring { offset, .. } => *offset,
        IrOp::Drop => 0,
    }
}

pub(super) fn map_native_runtime_error(err: native_runtime::NativeRuntimeError) -> RuntimeError {
    RuntimeError {
        message: err.message().to_string(),
        offset: Some(err.offset()),
        raised_value: None,
    }
}
