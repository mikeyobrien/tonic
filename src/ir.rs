use crate::parser::{Ast, BinaryOp, Expr};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct IrProgram {
    functions: Vec<IrFunction>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct IrFunction {
    name: String,
    params: Vec<String>,
    ops: Vec<IrOp>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
enum IrOp {
    ConstInt { value: i64 },
    Call { callee: String, argc: usize },
    AddInt,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringError {
    message: String,
    offset: usize,
}

impl LoweringError {
    fn unsupported(kind: &'static str, offset: usize) -> Self {
        Self {
            message: format!("unsupported expression for ir lowering: {kind}"),
            offset,
        }
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at offset {}", self.message, self.offset)
    }
}

impl std::error::Error for LoweringError {}

pub fn lower_ast_to_ir(ast: &Ast) -> Result<IrProgram, LoweringError> {
    let mut functions = Vec::new();

    for module in &ast.modules {
        for function in &module.functions {
            let mut ops = Vec::new();
            lower_expr(&function.body, &module.name, &mut ops)?;
            ops.push(IrOp::Return);

            functions.push(IrFunction {
                name: qualify_function_name(&module.name, &function.name),
                params: function
                    .params
                    .iter()
                    .map(|param| param.name().to_string())
                    .collect(),
                ops,
            });
        }
    }

    Ok(IrProgram { functions })
}

fn lower_expr(expr: &Expr, current_module: &str, ops: &mut Vec<IrOp>) -> Result<(), LoweringError> {
    match expr {
        Expr::Int { value, .. } => {
            ops.push(IrOp::ConstInt { value: *value });
            Ok(())
        }
        Expr::Call { callee, args, .. } => {
            for arg in args {
                lower_expr(arg, current_module, ops)?;
            }

            ops.push(IrOp::Call {
                callee: qualify_call_target(current_module, callee),
                argc: args.len(),
            });

            Ok(())
        }
        Expr::Binary {
            op: BinaryOp::Plus,
            left,
            right,
            ..
        } => {
            lower_expr(left, current_module, ops)?;
            lower_expr(right, current_module, ops)?;
            ops.push(IrOp::AddInt);
            Ok(())
        }
        Expr::Question { offset, .. } => Err(LoweringError::unsupported("question", *offset)),
        Expr::Pipe { offset, .. } => Err(LoweringError::unsupported("pipe", *offset)),
        Expr::Case { offset, .. } => Err(LoweringError::unsupported("case", *offset)),
    }
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn qualify_call_target(current_module: &str, callee: &str) -> String {
    if callee.contains('.') {
        callee.to_string()
    } else {
        qualify_function_name(current_module, callee)
    }
}

#[cfg(test)]
mod tests {
    use super::lower_ast_to_ir;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;

    #[test]
    fn lower_ast_emits_const_int_and_return_for_literal_function() {
        let source = "defmodule Demo do\n  def run() do\n    1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for literal body");

        assert_eq!(
            serde_json::to_string(&ir).expect("ir should serialize"),
            concat!(
                "{\"functions\":[",
                "{\"name\":\"Demo.run\",\"params\":[],\"ops\":[",
                "{\"op\":\"const_int\",\"value\":1},",
                "{\"op\":\"return\"}",
                "]}",
                "]}"
            )
        );
    }

    #[test]
    fn lower_ast_qualifies_local_call_targets() {
        let source = "defmodule Demo do\n  def run() do\n    helper(1)\n  end\n\n  def helper(value) do\n    value()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for call body");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {"op":"const_int","value":1},
                {"op":"call","callee":"Demo.helper","argc":1},
                {"op":"return"}
            ])
        );
    }
}
