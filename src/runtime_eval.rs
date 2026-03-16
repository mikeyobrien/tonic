use super::try_helper::evaluate_try;
use super::*;

pub(crate) fn evaluate_ops(
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
            IrOp::IntDiv { offset } => {
                let right = pop_value(stack, *offset, "div")?;
                let left = pop_value(stack, *offset, "div")?;
                let result = native_runtime::ops::int_div(left, right, *offset)
                    .map_err(map_native_runtime_error)?;
                stack.push(result);
            }
            IrOp::RemInt { offset } => {
                let right = pop_value(stack, *offset, "rem")?;
                let left = pop_value(stack, *offset, "rem")?;
                let result = native_runtime::ops::rem_int(left, right, *offset)
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
            IrOp::Drop => {
                stack.pop();
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
                if let Some(v) = evaluate_try(
                    program,
                    body_ops,
                    rescue_branches,
                    catch_branches,
                    after_ops,
                    env,
                    stack,
                )? {
                    return Ok(Some(v));
                }
            }
            IrOp::Raise { offset } => {
                let value = pop_value(stack, *offset, "raise")?;
                return Err(RuntimeError::raised(value, *offset));
            }
            IrOp::For {
                generators,
                body_ops,
                into_ops,
                reduce_ops,
                offset,
            } => {
                let result = evaluate_for(
                    program,
                    generators,
                    body_ops,
                    into_ops.as_deref(),
                    reduce_ops.as_deref(),
                    env,
                    *offset,
                )?;
                stack.push(result);
            }
            IrOp::Bitstring { count, offset } => {
                let mut bytes = Vec::with_capacity(*count);
                for _ in 0..*count {
                    bytes.push(pop_value(stack, *offset, "bitstring byte")?);
                }
                bytes.reverse();
                stack.push(RuntimeValue::List(bytes));
            }
        }
    }
    Ok(None)
}
