use crate::interop::{HostError, HOST_REGISTRY};
use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern, IrProgram};
use std::collections::HashMap;
use std::fmt;

const ENTRYPOINT: &str = "Demo.run";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Int(i64),
    Bool(bool),
    Nil,
    String(String),
    Atom(String),
    ResultOk(Box<RuntimeValue>),
    ResultErr(Box<RuntimeValue>),
    Tuple(Box<RuntimeValue>, Box<RuntimeValue>),
    Map(Box<RuntimeValue>, Box<RuntimeValue>),
    Keyword(Box<RuntimeValue>, Box<RuntimeValue>),
    List(Vec<RuntimeValue>),
    Range(i64, i64),
}

impl RuntimeValue {
    pub fn render(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Nil => "nil".to_string(),
            Self::String(value) => format!("\"{}\"", value),
            Self::Atom(value) => format!(":{value}"),
            Self::ResultOk(value) => format!("ok({})", value.render()),
            Self::ResultErr(value) => format!("err({})", value.render()),
            Self::Tuple(left, right) => format!("{{{}, {}}}", left.render(), right.render()),
            Self::Map(key, value) => format!("%{{{} => {}}}", key.render(), value.render()),
            Self::Keyword(key, value) => {
                let rendered_key = match key.as_ref() {
                    RuntimeValue::Atom(atom) => atom.clone(),
                    _ => key.render(),
                };
                format!("[{}: {}]", rendered_key, value.render())
            }
            Self::List(items) => {
                let items: Vec<String> = items.iter().map(|item| item.render()).collect();
                format!("[{}]", items.join(", "))
            }
            Self::Range(start, end) => format!("{}..{}", start, end),
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::Bool(_) => "bool",
            Self::Nil => "nil",
            Self::String(_) => "string",
            Self::Atom(_) => "atom",
            Self::ResultOk(_) | Self::ResultErr(_) => "result",
            Self::Tuple(_, _) => "tuple",
            Self::Map(_, _) => "map",
            Self::Keyword(_, _) => "keyword",
            Self::List(_) => "list",
            Self::Range(_, _) => "range",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
    offset: Option<usize>,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset: None,
        }
    }

    fn at_offset(message: impl Into<String>, offset: usize) -> Self {
        Self {
            message: message.into(),
            offset: Some(offset),
        }
    }
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
    evaluate_function(program, ENTRYPOINT, &[])
}

fn evaluate_function(
    program: &IrProgram,
    function_name: &str,
    args: &[RuntimeValue],
) -> Result<RuntimeValue, RuntimeError> {
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .ok_or_else(|| RuntimeError::new(format!("missing runtime function: {function_name}")))?;

    if function.params.len() != args.len() {
        return Err(RuntimeError::new(format!(
            "arity mismatch for runtime function {function_name}: expected {} args, found {}",
            function.params.len(),
            args.len()
        )));
    }

    let mut env = HashMap::new();
    for (param, arg) in function.params.iter().zip(args.iter()) {
        env.insert(param.clone(), arg.clone());
    }

    if let Some(guard_ops) = &function.guard_ops {
        let guard_passed = evaluate_guard_ops(program, guard_ops, &mut env)?;
        if !guard_passed {
            let guard_offset = guard_ops.first().map(ir_op_offset).unwrap_or(0);
            return Err(RuntimeError::at_offset(
                format!("no function clause matching {function_name}"),
                guard_offset,
            ));
        }
    }

    let mut stack: Vec<RuntimeValue> = Vec::new();

    if let Some(ret) = evaluate_ops(program, &function.ops, &mut env, &mut stack)? {
        return Ok(ret);
    }

    Err(RuntimeError::new(format!(
        "runtime function ended without return: {function_name}"
    )))
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
            IrOp::ConstBool { value, .. } => stack.push(RuntimeValue::Bool(*value)),
            IrOp::ConstNil { .. } => stack.push(RuntimeValue::Nil),
            IrOp::ConstString { value, .. } => stack.push(RuntimeValue::String(value.clone())),
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
            IrOp::Not { offset } => {
                let value = pop_value(stack, *offset, "not")?;
                match value {
                    RuntimeValue::Bool(flag) => stack.push(RuntimeValue::Bool(!flag)),
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::Bang { offset } => {
                let value = pop_value(stack, *offset, "!")?;
                let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));
                stack.push(RuntimeValue::Bool(!truthy));
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
                match (left, right) {
                    (RuntimeValue::String(l), RuntimeValue::String(r)) => {
                        stack.push(RuntimeValue::String(l + &r))
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::In { offset } => {
                let right = pop_value(stack, *offset, "in")?;
                let left = pop_value(stack, *offset, "in")?;

                let found = match right {
                    RuntimeValue::List(items) => items.contains(&left),
                    RuntimeValue::Range(start, end) => {
                        if let RuntimeValue::Int(val) = left {
                            val >= start && val <= end
                        } else {
                            false
                        }
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                };
                stack.push(RuntimeValue::Bool(found));
            }
            IrOp::PlusPlus { offset } => {
                let right = pop_value(stack, *offset, "++")?;
                let left = pop_value(stack, *offset, "++")?;
                match (left, right) {
                    (RuntimeValue::List(mut l), RuntimeValue::List(mut r)) => {
                        l.append(&mut r);
                        stack.push(RuntimeValue::List(l));
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::MinusMinus { offset } => {
                let right = pop_value(stack, *offset, "--")?;
                let left = pop_value(stack, *offset, "--")?;
                match (left, right) {
                    (RuntimeValue::List(mut l), RuntimeValue::List(r)) => {
                        for item in r {
                            if let Some(pos) = l.iter().position(|x| x == &item) {
                                l.remove(pos);
                            }
                        }
                        stack.push(RuntimeValue::List(l));
                    }
                    _ => return Err(RuntimeError::at_offset("badarg".to_string(), *offset)),
                }
            }
            IrOp::Range { offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                stack.push(RuntimeValue::Range(left, right));
            }
            IrOp::AddInt { offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                stack.push(RuntimeValue::Int(left + right));
            }
            IrOp::SubInt { offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                stack.push(RuntimeValue::Int(left - right));
            }
            IrOp::MulInt { offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                stack.push(RuntimeValue::Int(left * right));
            }
            IrOp::DivInt { offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                if right == 0 {
                    return Err(RuntimeError::at_offset("division by zero", *offset));
                }
                stack.push(RuntimeValue::Int(left / right));
            }
            IrOp::CmpInt { kind, offset } => {
                let right = pop_int(stack, *offset)?;
                let left = pop_int(stack, *offset)?;
                let result = match kind {
                    CmpKind::Eq => left == right,
                    CmpKind::NotEq => left != right,
                    CmpKind::Lt => left < right,
                    CmpKind::Lte => left <= right,
                    CmpKind::Gt => left > right,
                    CmpKind::Gte => left >= right,
                };
                stack.push(RuntimeValue::Bool(result));
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
                let mut matched = false;

                for branch in branches {
                    let mut bindings = HashMap::new();
                    if !match_pattern(&subject, &branch.pattern, env, &mut bindings) {
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
        | IrOp::ConstBool { offset, .. }
        | IrOp::ConstNil { offset }
        | IrOp::ConstString { offset, .. }
        | IrOp::Call { offset, .. }
        | IrOp::Question { offset }
        | IrOp::Case { offset, .. }
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
        IrPattern::List { items } => match value {
            RuntimeValue::List(values) if values.len() == items.len() => values
                .iter()
                .zip(items.iter())
                .all(|(value, pattern)| match_pattern(value, pattern, env, bindings)),
            _ => false,
        },
        IrPattern::Map { entries } => match value {
            RuntimeValue::Map(_, _) if entries.is_empty() => true,
            RuntimeValue::Map(key, map_value) if entries.len() == 1 => {
                match_pattern(key, &entries[0].key, env, bindings)
                    && match_pattern(map_value, &entries[0].value, env, bindings)
            }
            RuntimeValue::Map(_, _) => false,
            _ => false,
        },
    }
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
            let value = evaluate_function(program, name, &stack[args_start..])?;
            stack.truncate(args_start);
            Ok(value)
        }
    }
}

fn evaluate_builtin_call(
    name: &str,
    args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    match name {
        "ok" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            Ok(RuntimeValue::ResultOk(Box::new(arg)))
        }
        "err" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            Ok(RuntimeValue::ResultErr(Box::new(arg)))
        }
        "tuple" => {
            let (left, right) = expect_pair_builtin_args(name, args, offset)?;
            Ok(RuntimeValue::Tuple(Box::new(left), Box::new(right)))
        }
        "list" => Ok(RuntimeValue::List(args)),
        "map" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(RuntimeValue::Map(Box::new(key), Box::new(value)))
        }
        "keyword" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(RuntimeValue::Keyword(Box::new(key), Box::new(value)))
        }
        "protocol_dispatch" => {
            let value = expect_single_builtin_arg(name, args, offset)?;
            evaluate_protocol_dispatch(value, offset)
        }
        "host_call" => evaluate_host_call(args, offset),
        _ => Err(RuntimeError::at_offset(
            format!("unsupported builtin call in runtime evaluator: {name}"),
            offset,
        )),
    }
}

const PROTOCOL_DISPATCH_TABLE: &[(&str, i64)] = &[("tuple", 1), ("map", 2)];

fn evaluate_protocol_dispatch(
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    let implementation = PROTOCOL_DISPATCH_TABLE
        .iter()
        .find_map(|(kind, implementation)| (value.kind_label() == *kind).then_some(*implementation))
        .ok_or_else(|| {
            RuntimeError::at_offset(
                format!(
                    "protocol_dispatch has no implementation for {}",
                    value.kind_label()
                ),
                offset,
            )
        })?;

    Ok(RuntimeValue::Int(implementation))
}

fn evaluate_host_call(
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::at_offset(
            "host_call requires at least 1 argument (host function key)",
            offset,
        ));
    }

    // First argument must be the host function key (atom)
    let key = args.remove(0);
    let key_str = match key {
        RuntimeValue::Atom(s) => s,
        other => {
            return Err(RuntimeError::at_offset(
                format!(
                    "host_call first argument must be an atom (host key), found {}",
                    other.kind_label()
                ),
                offset,
            ));
        }
    };

    // Call the host function via registry
    HOST_REGISTRY
        .call(&key_str, &args)
        .map_err(|e: HostError| RuntimeError::at_offset(e.to_string(), offset))
}

fn expect_single_builtin_arg(
    name: &str,
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::at_offset(
            format!(
                "arity mismatch for runtime builtin {name}: expected 1 args, found {}",
                args.len()
            ),
            offset,
        ));
    }

    Ok(args
        .pop()
        .expect("arity check should guarantee one builtin argument"))
}

fn expect_pair_builtin_args(
    name: &str,
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<(RuntimeValue, RuntimeValue), RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::at_offset(
            format!(
                "arity mismatch for runtime builtin {name}: expected 2 args, found {}",
                args.len()
            ),
            offset,
        ));
    }

    let right = args
        .pop()
        .expect("arity check should guarantee second builtin argument");
    let left = args
        .pop()
        .expect("arity check should guarantee first builtin argument");

    Ok((left, right))
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

fn pop_int(stack: &mut Vec<RuntimeValue>, offset: usize) -> Result<i64, RuntimeError> {
    let value = pop_value(stack, offset, "int op")?;

    match value {
        RuntimeValue::Int(number) => Ok(number),
        other => Err(RuntimeError::at_offset(
            format!(
                "int operator expects int operands, found {}",
                other.kind_label()
            ),
            offset,
        )),
    }
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
