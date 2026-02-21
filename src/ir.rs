use crate::parser::{Ast, BinaryOp, Expr, Pattern};
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
    Question,
    Case { branches: Vec<IrCaseBranch> },
    AddInt,
    Return,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct IrCaseBranch {
    pattern: IrPattern,
    ops: Vec<IrOp>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum IrPattern {
    Atom { value: String },
    Bind { name: String },
    Wildcard,
    Tuple { items: Vec<IrPattern> },
    List { items: Vec<IrPattern> },
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
        Expr::Question { value, .. } => {
            lower_expr(value, current_module, ops)?;
            ops.push(IrOp::Question);
            Ok(())
        }
        Expr::Pipe { offset, .. } => Err(LoweringError::unsupported("pipe", *offset)),
        Expr::Case {
            subject,
            branches,
            offset,
            ..
        } => {
            lower_expr(subject, current_module, ops)?;

            let lowered_branches = branches
                .iter()
                .map(|branch| {
                    let mut branch_ops = Vec::new();
                    lower_expr(branch.body(), current_module, &mut branch_ops)?;

                    Ok(IrCaseBranch {
                        pattern: lower_pattern(branch.head(), *offset)?,
                        ops: branch_ops,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            ops.push(IrOp::Case {
                branches: lowered_branches,
            });
            Ok(())
        }
    }
}

fn lower_pattern(pattern: &Pattern, case_offset: usize) -> Result<IrPattern, LoweringError> {
    match pattern {
        Pattern::Atom { value } => Ok(IrPattern::Atom {
            value: value.clone(),
        }),
        Pattern::Bind { name } => Ok(IrPattern::Bind { name: name.clone() }),
        Pattern::Wildcard => Ok(IrPattern::Wildcard),
        Pattern::Tuple { items } => {
            let items = items
                .iter()
                .map(|item| lower_pattern(item, case_offset))
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::Tuple { items })
        }
        Pattern::List { items } => {
            let items = items
                .iter()
                .map(|item| lower_pattern(item, case_offset))
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::List { items })
        }
        Pattern::Map { .. } => Err(LoweringError::unsupported("map pattern", case_offset)),
    }
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn qualify_call_target(current_module: &str, callee: &str) -> String {
    if is_result_constructor_builtin(callee) || callee.contains('.') {
        callee.to_string()
    } else {
        qualify_function_name(current_module, callee)
    }
}

fn is_result_constructor_builtin(callee: &str) -> bool {
    matches!(callee, "ok" | "err")
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

    #[test]
    fn lower_ast_supports_question_and_case_ops() {
        let source = "defmodule Demo do\n  def run() do\n    case ok(1)? do\n      :ok -> 2\n      _ -> 3\n    end\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should support question and case");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {"op":"const_int","value":1},
                {"op":"call","callee":"ok","argc":1},
                {"op":"question"},
                {
                    "op":"case",
                    "branches":[
                        {
                            "pattern":{"kind":"atom","value":"ok"},
                            "ops":[{"op":"const_int","value":2}]
                        },
                        {
                            "pattern":{"kind":"wildcard"},
                            "ops":[{"op":"const_int","value":3}]
                        }
                    ]
                },
                {"op":"return"}
            ])
        );
    }
}
