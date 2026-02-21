use crate::ir::{IrCallTarget, IrOp, IrProgram};
use std::fmt;

const ENTRYPOINT: &str = "Demo.run";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Int(i64),
}

impl RuntimeValue {
    pub fn render(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
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

    let mut stack: Vec<RuntimeValue> = Vec::new();

    for op in &function.ops {
        match op {
            IrOp::ConstInt { value, .. } => stack.push(RuntimeValue::Int(*value)),
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                let call_args = pop_args(&mut stack, *argc, *offset)?;
                let value = evaluate_call(program, callee, &call_args, *offset)?;
                stack.push(value);
            }
            IrOp::AddInt { offset } => {
                let right = pop_int(&mut stack, *offset)?;
                let left = pop_int(&mut stack, *offset)?;
                stack.push(RuntimeValue::Int(left + right));
            }
            IrOp::Return { offset } => {
                return stack.pop().ok_or_else(|| {
                    RuntimeError::at_offset("runtime stack underflow on return", *offset)
                });
            }
            IrOp::Question { offset } => {
                return Err(RuntimeError::at_offset(
                    "unsupported ir op in runtime evaluator: question",
                    *offset,
                ));
            }
            IrOp::Case { offset, .. } => {
                return Err(RuntimeError::at_offset(
                    "unsupported ir op in runtime evaluator: case",
                    *offset,
                ));
            }
        }
    }

    Err(RuntimeError::new(format!(
        "runtime function ended without return: {function_name}"
    )))
}

fn evaluate_call(
    program: &IrProgram,
    callee: &IrCallTarget,
    args: &[RuntimeValue],
    offset: usize,
) -> Result<RuntimeValue, RuntimeError> {
    match callee {
        IrCallTarget::Builtin { name } => Err(RuntimeError::at_offset(
            format!("unsupported builtin call in runtime evaluator: {name}"),
            offset,
        )),
        IrCallTarget::Function { name } => evaluate_function(program, name, args),
    }
}

fn pop_args(
    stack: &mut Vec<RuntimeValue>,
    argc: usize,
    offset: usize,
) -> Result<Vec<RuntimeValue>, RuntimeError> {
    if stack.len() < argc {
        return Err(RuntimeError::at_offset(
            format!("runtime stack underflow for call with {argc} args"),
            offset,
        ));
    }

    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(
            stack
                .pop()
                .expect("stack length should be validated before popping call args"),
        );
    }

    args.reverse();
    Ok(args)
}

fn pop_int(stack: &mut Vec<RuntimeValue>, offset: usize) -> Result<i64, RuntimeError> {
    let Some(value) = stack.pop() else {
        return Err(RuntimeError::at_offset(
            "runtime stack underflow for add_int",
            offset,
        ));
    };

    let RuntimeValue::Int(number) = value;
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::{evaluate_entrypoint, RuntimeError, RuntimeValue};
    use crate::ir::lower_ast_to_ir;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;

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
}
