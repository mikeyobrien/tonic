use crate::ir::{IrCallTarget, IrOp, IrProgram};
use std::fmt;

const ENTRYPOINT: &str = "Demo.run";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Int(i64),
    ResultOk(Box<RuntimeValue>),
    ResultErr(Box<RuntimeValue>),
    Tuple(Box<RuntimeValue>, Box<RuntimeValue>),
    Map(Box<RuntimeValue>, Box<RuntimeValue>),
    Keyword(Box<RuntimeValue>, Box<RuntimeValue>),
}

impl RuntimeValue {
    pub fn render(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::ResultOk(value) => format!("ok({})", value.render()),
            Self::ResultErr(value) => format!("err({})", value.render()),
            Self::Tuple(left, right) => format!("{{{}, {}}}", left.render(), right.render()),
            Self::Map(key, value) => format!("%{{{} => {}}}", key.render(), value.render()),
            Self::Keyword(key, value) => format!("[{}: {}]", key.render(), value.render()),
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::ResultOk(_) | Self::ResultErr(_) => "result",
            Self::Tuple(_, _) => "tuple",
            Self::Map(_, _) => "map",
            Self::Keyword(_, _) => "keyword",
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
                let value = evaluate_call(program, callee, &mut stack, *argc, *offset)?;
                stack.push(value);
            }
            IrOp::AddInt { offset } => {
                let right = pop_int(&mut stack, *offset)?;
                let left = pop_int(&mut stack, *offset)?;
                stack.push(RuntimeValue::Int(left + right));
            }
            IrOp::Return { offset } => {
                return pop_value(&mut stack, *offset, "return");
            }
            IrOp::Question { offset } => {
                let value = pop_value(&mut stack, *offset, "question")?;

                match value {
                    RuntimeValue::ResultOk(inner) => stack.push(*inner),
                    RuntimeValue::ResultErr(inner) => {
                        return Ok(RuntimeValue::ResultErr(inner));
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
        "map" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(RuntimeValue::Map(Box::new(key), Box::new(value)))
        }
        "keyword" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(RuntimeValue::Keyword(Box::new(key), Box::new(value)))
        }
        _ => Err(RuntimeError::at_offset(
            format!("unsupported builtin call in runtime evaluator: {name}"),
            offset,
        )),
    }
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
    let value = pop_value(stack, offset, "add_int")?;

    match value {
        RuntimeValue::Int(number) => Ok(number),
        other => Err(RuntimeError::at_offset(
            format!("add_int expects int operands, found {}", other.kind_label()),
            offset,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_builtin_call, evaluate_entrypoint, RuntimeError, RuntimeValue};
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
}
