use super::*;

pub(super) fn evaluate_try(
    program: &IrProgram,
    body_ops: &[IrOp],
    rescue_branches: &[IrCaseBranch],
    catch_branches: &[IrCaseBranch],
    after_ops: &Option<Vec<IrOp>>,
    env: &mut HashMap<String, RuntimeValue>,
    stack: &mut Vec<RuntimeValue>,
) -> Result<Option<RuntimeValue>, RuntimeError> {
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
                .unwrap_or_else(|| RuntimeValue::String(err.to_string()));

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
                    let guard_passed = evaluate_guard_ops(program, guard_ops, &mut branch_env)?;
                    if !guard_passed {
                        continue;
                    }
                }

                let mut branch_stack = Vec::new();
                match evaluate_ops(program, &branch.ops, &mut branch_env, &mut branch_stack) {
                    Ok(ret) => {
                        if let Some(v) = ret {
                            early_return = Some(v);
                        } else {
                            let result = branch_stack
                                .pop()
                                .unwrap_or_else(|| RuntimeValue::Atom("ok".to_string()));
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
                        let guard_passed = evaluate_guard_ops(program, guard_ops, &mut branch_env)?;
                        if !guard_passed {
                            continue;
                        }
                    }

                    let mut branch_stack = Vec::new();
                    match evaluate_ops(program, &branch.ops, &mut branch_env, &mut branch_stack) {
                        Ok(ret) => {
                            if let Some(v) = ret {
                                early_return = Some(v);
                            } else {
                                let result = branch_stack
                                    .pop()
                                    .unwrap_or_else(|| RuntimeValue::Atom("ok".to_string()));
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

    Ok(early_return)
}
