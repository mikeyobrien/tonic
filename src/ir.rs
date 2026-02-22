use crate::parser::{Ast, BinaryOp, Expr, Pattern};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IrProgram {
    pub(crate) functions: Vec<IrFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IrFunction {
    pub(crate) name: String,
    pub(crate) params: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) param_patterns: Option<Vec<IrPattern>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) guard_ops: Option<Vec<IrOp>>,
    pub(crate) ops: Vec<IrOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum IrOp {
    ConstInt {
        value: i64,
        offset: usize,
    },
    ConstBool {
        value: bool,
        offset: usize,
    },
    ConstNil {
        offset: usize,
    },
    ConstString {
        value: String,
        offset: usize,
    },
    Call {
        callee: IrCallTarget,
        argc: usize,
        offset: usize,
    },
    MakeClosure {
        params: Vec<String>,
        ops: Vec<IrOp>,
        offset: usize,
    },
    CallValue {
        argc: usize,
        offset: usize,
    },
    Question {
        offset: usize,
    },
    Case {
        branches: Vec<IrCaseBranch>,
        offset: usize,
    },
    LoadVariable {
        name: String,
        offset: usize,
    },
    ConstAtom {
        value: String,
        offset: usize,
    },
    AddInt {
        offset: usize,
    },
    SubInt {
        offset: usize,
    },
    MulInt {
        offset: usize,
    },
    DivInt {
        offset: usize,
    },
    CmpInt {
        kind: CmpKind,
        offset: usize,
    },
    Not {
        offset: usize,
    },
    Bang {
        offset: usize,
    },
    AndAnd {
        right_ops: Vec<IrOp>,
        offset: usize,
    },
    OrOr {
        right_ops: Vec<IrOp>,
        offset: usize,
    },
    And {
        right_ops: Vec<IrOp>,
        offset: usize,
    },
    Or {
        right_ops: Vec<IrOp>,
        offset: usize,
    },
    Concat {
        offset: usize,
    },
    In {
        offset: usize,
    },
    PlusPlus {
        offset: usize,
    },
    MinusMinus {
        offset: usize,
    },
    Range {
        offset: usize,
    },
    Match {
        pattern: IrPattern,
        offset: usize,
    },
    Return {
        offset: usize,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CmpKind {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum IrCallTarget {
    Builtin { name: String },
    Function { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IrCaseBranch {
    pub(crate) pattern: IrPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) guard_ops: Option<Vec<IrOp>>,
    pub(crate) ops: Vec<IrOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum IrPattern {
    Atom { value: String },
    Bind { name: String },
    Pin { name: String },
    Wildcard,
    Integer { value: i64 },
    Tuple { items: Vec<IrPattern> },
    List { items: Vec<IrPattern> },
    Map { entries: Vec<IrMapPatternEntry> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IrMapPatternEntry {
    pub(crate) key: IrPattern,
    pub(crate) value: IrPattern,
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
            ops.push(IrOp::Return {
                offset: function.body.offset(),
            });

            let guard_ops = if let Some(guard) = function.guard() {
                let mut guard_ops = Vec::new();
                lower_expr(guard, &module.name, &mut guard_ops)?;
                Some(guard_ops)
            } else {
                None
            };

            let params = function
                .params
                .iter()
                .map(|param| param.name().to_string())
                .collect::<Vec<_>>();
            let param_patterns = lower_function_param_patterns(function, &params)?;

            functions.push(IrFunction {
                name: qualify_function_name(&module.name, &function.name),
                params,
                param_patterns,
                guard_ops,
                ops,
            });

            let wrappers = lower_default_argument_wrappers(module.name.as_str(), function)?;
            functions.extend(wrappers);
        }
    }

    Ok(IrProgram { functions })
}

fn lower_function_param_patterns(
    function: &crate::parser::Function,
    params: &[String],
) -> Result<Option<Vec<IrPattern>>, LoweringError> {
    let lowered_patterns = function
        .params
        .iter()
        .map(|param| lower_pattern(param.pattern()))
        .collect::<Result<Vec<_>, _>>()?;

    let is_simple_bind_shape = lowered_patterns.iter().zip(params.iter()).all(
        |(pattern, param_name)| matches!(pattern, IrPattern::Bind { name } if name == param_name),
    );

    if is_simple_bind_shape {
        Ok(None)
    } else {
        Ok(Some(lowered_patterns))
    }
}

fn lower_default_argument_wrappers(
    module_name: &str,
    function: &crate::parser::Function,
) -> Result<Vec<IrFunction>, LoweringError> {
    let trailing_default_count = function
        .params
        .iter()
        .rev()
        .take_while(|param| param.has_default())
        .count();

    if trailing_default_count == 0 {
        return Ok(Vec::new());
    }

    let total_params = function.params.len();
    let min_arity = total_params - trailing_default_count;
    let qualified_name = qualify_function_name(module_name, &function.name);
    let call_offset = function.body.offset();
    let mut wrappers = Vec::new();

    for provided_arity in min_arity..total_params {
        let mut ops = Vec::new();
        let wrapper_params = function.params[..provided_arity]
            .iter()
            .map(|param| param.name().to_string())
            .collect::<Vec<_>>();

        for param in &wrapper_params {
            ops.push(IrOp::LoadVariable {
                name: param.clone(),
                offset: call_offset,
            });
        }

        for parameter in &function.params[provided_arity..] {
            let default = parameter
                .default()
                .ok_or_else(|| LoweringError::unsupported("default argument shape", call_offset))?;
            lower_expr(default, module_name, &mut ops)?;
        }

        ops.push(IrOp::Call {
            callee: IrCallTarget::Function {
                name: qualified_name.clone(),
            },
            argc: total_params,
            offset: call_offset,
        });
        ops.push(IrOp::Return {
            offset: call_offset,
        });

        wrappers.push(IrFunction {
            name: qualified_name.clone(),
            params: wrapper_params,
            param_patterns: None,
            guard_ops: None,
            ops,
        });
    }

    Ok(wrappers)
}

fn lower_expr(expr: &Expr, current_module: &str, ops: &mut Vec<IrOp>) -> Result<(), LoweringError> {
    match expr {
        Expr::Int { value, offset, .. } => {
            ops.push(IrOp::ConstInt {
                value: *value,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Bool { value, offset, .. } => {
            ops.push(IrOp::ConstBool {
                value: *value,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Nil { offset, .. } => {
            ops.push(IrOp::ConstNil { offset: *offset });
            Ok(())
        }
        Expr::String { value, offset, .. } => {
            ops.push(IrOp::ConstString {
                value: value.clone(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Tuple { items, offset, .. } => {
            if items.len() != 2 {
                return Err(LoweringError::unsupported("tuple literal arity", *offset));
            }

            for item in items {
                lower_expr(item, current_module, ops)?;
            }

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "tuple".to_string(),
                },
                argc: 2,
                offset: *offset,
            });
            Ok(())
        }
        Expr::List { items, offset, .. } => {
            for item in items {
                lower_expr(item, current_module, ops)?;
            }

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "list".to_string(),
                },
                argc: items.len(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Map {
            entries, offset, ..
        } => {
            if entries.len() != 1 {
                return Err(LoweringError::unsupported("map literal arity", *offset));
            }

            let entry = &entries[0];
            ops.push(IrOp::ConstAtom {
                value: entry.key.clone(),
                offset: *offset,
            });
            lower_expr(&entry.value, current_module, ops)?;

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "map".to_string(),
                },
                argc: 2,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Keyword {
            entries, offset, ..
        } => {
            if entries.len() != 1 {
                return Err(LoweringError::unsupported("keyword literal arity", *offset));
            }

            let entry = &entries[0];
            ops.push(IrOp::ConstAtom {
                value: entry.key.clone(),
                offset: *offset,
            });
            lower_expr(&entry.value, current_module, ops)?;

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "keyword".to_string(),
                },
                argc: 2,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Call {
            callee,
            args,
            offset,
            ..
        } => {
            for arg in args {
                lower_expr(arg, current_module, ops)?;
            }

            ops.push(IrOp::Call {
                callee: qualify_call_target(current_module, callee),
                argc: args.len(),
                offset: *offset,
            });

            Ok(())
        }
        Expr::Fn {
            params,
            body,
            offset,
            ..
        } => {
            let mut closure_ops = Vec::new();
            lower_expr(body, current_module, &mut closure_ops)?;
            closure_ops.push(IrOp::Return {
                offset: body.offset(),
            });

            ops.push(IrOp::MakeClosure {
                params: params.clone(),
                ops: closure_ops,
                offset: *offset,
            });

            Ok(())
        }
        Expr::Invoke {
            callee,
            args,
            offset,
            ..
        } => {
            lower_expr(callee, current_module, ops)?;

            for arg in args {
                lower_expr(arg, current_module, ops)?;
            }

            ops.push(IrOp::CallValue {
                argc: args.len(),
                offset: *offset,
            });

            Ok(())
        }
        Expr::Unary {
            op, value, offset, ..
        } => {
            lower_expr(value, current_module, ops)?;
            let ir_op = match op {
                crate::parser::UnaryOp::Not => IrOp::Not { offset: *offset },
                crate::parser::UnaryOp::Bang => IrOp::Bang { offset: *offset },
            };
            ops.push(ir_op);
            Ok(())
        }
        Expr::Binary {
            op,
            left,
            right,
            offset,
            ..
        } => {
            if *op == BinaryOp::Match {
                lower_expr(right, current_module, ops)?;
                let pattern = lower_expr_pattern(left)?;
                ops.push(IrOp::Match {
                    pattern,
                    offset: *offset,
                });
                return Ok(());
            }

            lower_expr(left, current_module, ops)?;

            match op {
                BinaryOp::AndAnd => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, &mut right_ops)?;
                    ops.push(IrOp::AndAnd {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::OrOr => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, &mut right_ops)?;
                    ops.push(IrOp::OrOr {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::And => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, &mut right_ops)?;
                    ops.push(IrOp::And {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::Or => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, &mut right_ops)?;
                    ops.push(IrOp::Or {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                _ => {}
            }

            lower_expr(right, current_module, ops)?;
            let ir_op = match op {
                BinaryOp::Plus => IrOp::AddInt { offset: *offset },
                BinaryOp::Minus => IrOp::SubInt { offset: *offset },
                BinaryOp::Mul => IrOp::MulInt { offset: *offset },
                BinaryOp::Div => IrOp::DivInt { offset: *offset },
                BinaryOp::Eq => IrOp::CmpInt {
                    kind: CmpKind::Eq,
                    offset: *offset,
                },
                BinaryOp::NotEq => IrOp::CmpInt {
                    kind: CmpKind::NotEq,
                    offset: *offset,
                },
                BinaryOp::Lt => IrOp::CmpInt {
                    kind: CmpKind::Lt,
                    offset: *offset,
                },
                BinaryOp::Lte => IrOp::CmpInt {
                    kind: CmpKind::Lte,
                    offset: *offset,
                },
                BinaryOp::Gt => IrOp::CmpInt {
                    kind: CmpKind::Gt,
                    offset: *offset,
                },
                BinaryOp::Gte => IrOp::CmpInt {
                    kind: CmpKind::Gte,
                    offset: *offset,
                },
                BinaryOp::Concat => IrOp::Concat { offset: *offset },
                BinaryOp::In => IrOp::In { offset: *offset },
                BinaryOp::PlusPlus => IrOp::PlusPlus { offset: *offset },
                BinaryOp::MinusMinus => IrOp::MinusMinus { offset: *offset },
                BinaryOp::Range => IrOp::Range { offset: *offset },
                BinaryOp::Match
                | BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::AndAnd
                | BinaryOp::OrOr => unreachable!(),
            };
            ops.push(ir_op);
            Ok(())
        }
        Expr::Question { value, offset, .. } => {
            lower_expr(value, current_module, ops)?;
            ops.push(IrOp::Question { offset: *offset });
            Ok(())
        }
        Expr::Pipe {
            left,
            right,
            offset,
            ..
        } => lower_pipe_expr(left, right, *offset, current_module, ops),
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

                    let guard_ops = if let Some(guard) = branch.guard() {
                        let mut guard_ops = Vec::new();
                        lower_expr(guard, current_module, &mut guard_ops)?;
                        Some(guard_ops)
                    } else {
                        None
                    };

                    Ok(IrCaseBranch {
                        pattern: lower_pattern(branch.head())?,
                        guard_ops,
                        ops: branch_ops,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            ops.push(IrOp::Case {
                branches: lowered_branches,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Group { inner, .. } => lower_expr(inner, current_module, ops),
        Expr::Variable { name, offset, .. } => {
            ops.push(IrOp::LoadVariable {
                name: name.clone(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Atom { value, offset, .. } => {
            ops.push(IrOp::ConstAtom {
                value: value.clone(),
                offset: *offset,
            });
            Ok(())
        }
    }
}

fn lower_pipe_expr(
    left: &Expr,
    right: &Expr,
    pipe_offset: usize,
    current_module: &str,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    lower_expr(left, current_module, ops)?;

    let Expr::Call {
        callee,
        args,
        offset,
        ..
    } = right
    else {
        return Err(LoweringError::unsupported("pipe target", pipe_offset));
    };

    for arg in args {
        lower_expr(arg, current_module, ops)?;
    }

    ops.push(IrOp::Call {
        callee: qualify_call_target(current_module, callee),
        argc: args.len() + 1,
        offset: *offset,
    });

    Ok(())
}

fn lower_expr_pattern(expr: &Expr) -> Result<IrPattern, LoweringError> {
    match expr {
        Expr::Atom { value, .. } => Ok(IrPattern::Atom {
            value: value.clone(),
        }),
        Expr::Variable { name, .. } if name == "_" => Ok(IrPattern::Wildcard),
        Expr::Variable { name, .. } => Ok(IrPattern::Bind { name: name.clone() }),
        Expr::Int { value, .. } => Ok(IrPattern::Integer { value: *value }),
        Expr::Tuple { items, offset, .. } => {
            let items = items
                .iter()
                .map(lower_expr_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;
            if items.len() != 2 {
                return Err(LoweringError::unsupported(
                    "match tuple pattern arity",
                    *offset,
                ));
            }
            Ok(IrPattern::Tuple { items })
        }
        Expr::List { items, .. } => {
            let items = items
                .iter()
                .map(lower_expr_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;
            Ok(IrPattern::List { items })
        }
        Expr::Map { entries, .. } => {
            let entries = entries
                .iter()
                .map(|entry| {
                    Ok(IrMapPatternEntry {
                        key: IrPattern::Atom {
                            value: entry.key.clone(),
                        },
                        value: lower_expr_pattern(&entry.value)?,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;
            Ok(IrPattern::Map { entries })
        }
        Expr::Binary {
            op: BinaryOp::Match,
            ..
        } => Err(LoweringError::unsupported(
            "nested match pattern",
            expr.offset(),
        )),
        _ => Err(LoweringError::unsupported("match pattern", expr.offset())),
    }
}

fn lower_pattern(pattern: &Pattern) -> Result<IrPattern, LoweringError> {
    match pattern {
        Pattern::Atom { value } => Ok(IrPattern::Atom {
            value: value.clone(),
        }),
        Pattern::Bind { name } => Ok(IrPattern::Bind { name: name.clone() }),
        Pattern::Pin { name } => Ok(IrPattern::Pin { name: name.clone() }),
        Pattern::Wildcard => Ok(IrPattern::Wildcard),
        Pattern::Integer { value } => Ok(IrPattern::Integer { value: *value }),
        Pattern::Tuple { items } => {
            let items = items
                .iter()
                .map(lower_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::Tuple { items })
        }
        Pattern::List { items } => {
            let items = items
                .iter()
                .map(lower_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::List { items })
        }
        Pattern::Map { entries } => {
            let entries = entries
                .iter()
                .map(|entry| {
                    Ok(IrMapPatternEntry {
                        key: lower_pattern(entry.key())?,
                        value: lower_pattern(entry.value())?,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::Map { entries })
        }
    }
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn qualify_call_target(current_module: &str, callee: &str) -> IrCallTarget {
    if is_builtin_call_target(callee) {
        IrCallTarget::Builtin {
            name: callee.to_string(),
        }
    } else if callee.contains('.') {
        IrCallTarget::Function {
            name: callee.to_string(),
        }
    } else {
        IrCallTarget::Function {
            name: qualify_function_name(current_module, callee),
        }
    }
}

fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err" | "tuple" | "list" | "map" | "keyword" | "protocol_dispatch" | "host_call"
    )
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
                "{\"op\":\"const_int\",\"value\":1,\"offset\":37},",
                "{\"op\":\"return\",\"offset\":37}",
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
                {"op":"const_int","value":1,"offset":44},
                {"op":"call","callee":{"kind":"function","name":"Demo.helper"},"argc":1,"offset":37},
                {"op":"return","offset":37}
            ])
        );
    }

    #[test]
    fn lower_ast_canonicalizes_call_target_kinds() {
        let source = "defmodule Demo do\n  def run() do\n    ok(helper(1))\n  end\n\n  def helper(value) do\n    value()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for call body");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {"op":"const_int","value":1,"offset":47},
                {"op":"call","callee":{"kind":"function","name":"Demo.helper"},"argc":1,"offset":40},
                {"op":"call","callee":{"kind":"builtin","name":"ok"},"argc":1,"offset":37},
                {"op":"return","offset":37}
            ])
        );
    }

    #[test]
    fn lower_ast_marks_protocol_dispatch_as_builtin_call_target() {
        let source =
            "defmodule Demo do\n  def run() do\n    protocol_dispatch(tuple(1, 2))\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir =
            lower_ast_to_ir(&ast).expect("lowering should classify protocol dispatch as builtin");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {"op":"const_int","value":1,"offset":61},
                {"op":"const_int","value":2,"offset":64},
                {"op":"call","callee":{"kind":"builtin","name":"tuple"},"argc":2,"offset":55},
                {"op":"call","callee":{"kind":"builtin","name":"protocol_dispatch"},"argc":1,"offset":37},
                {"op":"return","offset":37}
            ])
        );
    }

    #[test]
    fn lower_ast_marks_host_call_as_builtin_call_target() {
        let source =
            "defmodule Demo do\n  def run() do\n    host_call(:identity, 42)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should classify host_call as builtin");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        // Find the host_call operation
        let ops = &json["functions"][0]["ops"];
        let host_call_op = ops
            .as_array()
            .unwrap()
            .iter()
            .find(|op| op["op"] == "call" && op["callee"]["name"] == "host_call")
            .expect("lowered ir should include host_call as builtin");

        assert_eq!(host_call_op["callee"]["kind"], "builtin");
        assert_eq!(host_call_op["callee"]["name"], "host_call");
    }

    #[test]
    fn lower_ast_threads_pipe_input_into_rhs_call_arguments() {
        let source = "defmodule Enum do\n  def stage_one(_value) do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    tuple(1, 2) |> Enum.stage_one()\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should support pipe expressions");
        let run_function = ir
            .functions
            .iter()
            .find(|function| function.name == "Demo.run")
            .expect("lowered ir should include Demo.run");

        assert!(matches!(
            &run_function.ops[2],
            super::IrOp::Call {
                callee: super::IrCallTarget::Builtin { name },
                argc: 2,
                ..
            } if name == "tuple"
        ));

        assert!(matches!(
            &run_function.ops[3],
            super::IrOp::Call {
                callee: super::IrCallTarget::Function { name },
                argc: 1,
                ..
            } if name == "Enum.stage_one"
        ));
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
                {"op":"const_int","value":1,"offset":45},
                {"op":"call","callee":{"kind":"builtin","name":"ok"},"argc":1,"offset":42},
                {"op":"question","offset":47},
                {
                    "op":"case",
                    "branches":[
                        {
                            "pattern":{"kind":"atom","value":"ok"},
                            "ops":[{"op":"const_int","value":2,"offset":65}]
                        },
                        {
                            "pattern":{"kind":"wildcard"},
                            "ops":[{"op":"const_int","value":3,"offset":78}]
                        }
                    ],
                    "offset":37
                },
                {"op":"return","offset":37}
            ])
        );
    }

    #[test]
    fn lower_ast_emits_distinct_not_and_bang_ops() {
        let source = "defmodule Demo do\n  def run() do\n    tuple(not false, !nil)\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize unary op fixture");
        let ast = parse_ast(&tokens).expect("parser should build unary op fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should support unary op fixture");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {"op":"const_bool","value":false,"offset":47},
                {"op":"not","offset":43},
                {"op":"const_nil","offset":55},
                {"op":"bang","offset":54},
                {"op":"call","callee":{"kind":"builtin","name":"tuple"},"argc":2,"offset":37},
                {"op":"return","offset":37}
            ])
        );
    }
}
