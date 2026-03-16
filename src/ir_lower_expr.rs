use super::*;

#[path = "ir_lower_arms.rs"]
mod arms;
use arms::*;

pub(super) fn lower_expr(
    expr: &Expr,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    match expr {
        Expr::Int { value, offset, .. } => {
            ops.push(IrOp::ConstInt {
                value: *value,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Float { value, offset, .. } => {
            ops.push(IrOp::ConstFloat {
                value: value.clone(),
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
        Expr::InterpolatedString {
            segments, offset, ..
        } => {
            if segments.is_empty() {
                ops.push(IrOp::ConstString {
                    value: String::new(),
                    offset: *offset,
                });
                return Ok(());
            }

            for (i, segment) in segments.iter().enumerate() {
                match segment {
                    crate::parser::InterpolationSegment::String { value } => {
                        ops.push(IrOp::ConstString {
                            value: value.clone(),
                            offset: *offset,
                        });
                    }
                    crate::parser::InterpolationSegment::Expr { expr } => {
                        lower_expr(expr, current_module, struct_definitions, ops)?;
                        ops.push(IrOp::ToString { offset: *offset });
                    }
                }

                if i > 0 {
                    ops.push(IrOp::Concat { offset: *offset });
                }
            }
            Ok(())
        }
        Expr::Tuple { items, offset, .. } => {
            if items.len() != 2 {
                return Err(LoweringError::unsupported("tuple literal arity", *offset));
            }

            for item in items {
                lower_expr(item, current_module, struct_definitions, ops)?;
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
                lower_expr(item, current_module, struct_definitions, ops)?;
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
        Expr::Map { .. }
        | Expr::Struct { .. }
        | Expr::MapUpdate { .. }
        | Expr::StructUpdate { .. }
        | Expr::Keyword { .. } => {
            lower_collection_literals(expr, current_module, struct_definitions, ops)
        }
        Expr::Call {
            callee,
            args,
            offset,
            ..
        } => {
            for arg in args {
                lower_expr(arg, current_module, struct_definitions, ops)?;
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
            lower_expr(body, current_module, struct_definitions, &mut closure_ops)?;
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
            lower_expr(callee, current_module, struct_definitions, ops)?;

            for arg in args {
                lower_expr(arg, current_module, struct_definitions, ops)?;
            }

            ops.push(IrOp::CallValue {
                argc: args.len(),
                offset: *offset,
            });

            Ok(())
        }
        Expr::FieldAccess {
            base,
            label,
            offset,
            ..
        } => {
            // Special case: __ENV__.module resolves to the current module atom
            if let Expr::Variable { name, .. } = base.as_ref() {
                if name == "__ENV__" && label == "module" {
                    ops.push(IrOp::ConstAtom {
                        value: current_module.to_string(),
                        offset: *offset,
                    });
                    return Ok(());
                }
            }
            lower_expr(base, current_module, struct_definitions, ops)?;
            ops.push(IrOp::ConstAtom {
                value: label.clone(),
                offset: *offset,
            });

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "map_access".to_string(),
                },
                argc: 2,
                offset: *offset,
            });

            Ok(())
        }
        Expr::IndexAccess {
            base,
            index,
            offset,
            ..
        } => {
            lower_expr(base, current_module, struct_definitions, ops)?;
            lower_expr(index, current_module, struct_definitions, ops)?;

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "map_access".to_string(),
                },
                argc: 2,
                offset: *offset,
            });

            Ok(())
        }
        Expr::Unary {
            op, value, offset, ..
        } => {
            match op {
                crate::parser::UnaryOp::Minus => {
                    ops.push(IrOp::ConstInt {
                        value: 0,
                        offset: *offset,
                    });
                    lower_expr(value, current_module, struct_definitions, ops)?;
                    ops.push(IrOp::SubInt { offset: *offset });
                }
                crate::parser::UnaryOp::Plus => {
                    lower_expr(value, current_module, struct_definitions, ops)?;
                }
                crate::parser::UnaryOp::Not => {
                    lower_expr(value, current_module, struct_definitions, ops)?;
                    ops.push(IrOp::Not { offset: *offset });
                }
                crate::parser::UnaryOp::Bang => {
                    lower_expr(value, current_module, struct_definitions, ops)?;
                    ops.push(IrOp::Bang { offset: *offset });
                }
                crate::parser::UnaryOp::BitwiseNot => {
                    lower_expr(value, current_module, struct_definitions, ops)?;
                    ops.push(IrOp::BitwiseNot { offset: *offset });
                }
            }
            Ok(())
        }
        Expr::Binary {
            op,
            left,
            right,
            offset,
            ..
        } => lower_binary_expr(
            op,
            left,
            right,
            *offset,
            current_module,
            struct_definitions,
            ops,
        ),
        Expr::Question { value, offset, .. } => {
            lower_expr(value, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Question { offset: *offset });
            Ok(())
        }
        Expr::Pipe {
            left,
            right,
            offset,
            ..
        } => lower_pipe_expr(
            left,
            right,
            *offset,
            current_module,
            struct_definitions,
            ops,
        ),
        Expr::Case {
            subject,
            branches,
            offset,
            ..
        } => {
            lower_expr(subject, current_module, struct_definitions, ops)?;

            let lowered_branches = branches
                .iter()
                .map(|branch| {
                    let mut branch_ops = Vec::new();
                    lower_expr(
                        branch.body(),
                        current_module,
                        struct_definitions,
                        &mut branch_ops,
                    )?;

                    let guard_ops = if let Some(guard) = branch.guard() {
                        let mut guard_ops = Vec::new();
                        lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
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
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            offset,
            ..
        } => lower_try_expr(
            body,
            rescue,
            catch,
            after,
            *offset,
            current_module,
            struct_definitions,
            ops,
        ),
        Expr::Raise { error, offset, .. } => {
            lower_expr(error, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Raise { offset: *offset });
            Ok(())
        }
        Expr::For {
            generators,
            into,
            reduce,
            body,
            offset,
            ..
        } => lower_for_expr(
            generators,
            into,
            reduce,
            body,
            *offset,
            current_module,
            struct_definitions,
            ops,
        ),
        Expr::Group { inner, .. } => lower_expr(inner, current_module, struct_definitions, ops),
        Expr::Bitstring { items, offset, .. } => {
            for item in items {
                lower_expr(item, current_module, struct_definitions, ops)?;
            }
            ops.push(IrOp::Bitstring {
                count: items.len(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Variable { name, offset, .. } => {
            // __MODULE__ resolves to the current module's atom at compile time
            if name == "__MODULE__" {
                ops.push(IrOp::ConstAtom {
                    value: current_module.to_string(),
                    offset: *offset,
                });
                return Ok(());
            }
            // __ENV__ resolves to a map with :module key; full map is complex,
            // but bare __ENV__ emits a placeholder atom
            if name == "__ENV__" {
                ops.push(IrOp::ConstAtom {
                    value: current_module.to_string(),
                    offset: *offset,
                });
                return Ok(());
            }
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
        Expr::Block { exprs, .. } => {
            for (i, sub_expr) in exprs.iter().enumerate() {
                lower_expr(sub_expr, current_module, struct_definitions, ops)?;
                // Drop intermediate values (all but the last)
                if i < exprs.len() - 1 {
                    ops.push(IrOp::Drop);
                }
            }
            Ok(())
        }
    }
}
