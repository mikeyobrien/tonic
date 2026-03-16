use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_binary_expr(
    op: &crate::parser::BinaryOp,
    left: &Expr,
    right: &Expr,
    offset: usize,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    if *op == BinaryOp::Match {
        lower_expr(right, current_module, struct_definitions, ops)?;
        let pattern = lower_expr_pattern(left)?;
        ops.push(IrOp::Match { pattern, offset });
        return Ok(());
    }

    lower_expr(left, current_module, struct_definitions, ops)?;

    match op {
        BinaryOp::AndAnd => {
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::AndAnd { right_ops, offset });
            return Ok(());
        }
        BinaryOp::OrOr => {
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::OrOr { right_ops, offset });
            return Ok(());
        }
        BinaryOp::And => {
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::And { right_ops, offset });
            return Ok(());
        }
        BinaryOp::Or => {
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::Or { right_ops, offset });
            return Ok(());
        }
        _ => {}
    }

    lower_expr(right, current_module, struct_definitions, ops)?;
    let ir_op = match op {
        BinaryOp::Plus => IrOp::AddInt { offset },
        BinaryOp::Minus => IrOp::SubInt { offset },
        BinaryOp::Mul => IrOp::MulInt { offset },
        BinaryOp::Div => IrOp::DivInt { offset },
        BinaryOp::IntDiv => IrOp::IntDiv { offset },
        BinaryOp::Rem => IrOp::RemInt { offset },
        BinaryOp::Eq => IrOp::CmpInt {
            kind: CmpKind::Eq,
            offset,
        },
        BinaryOp::NotEq => IrOp::CmpInt {
            kind: CmpKind::NotEq,
            offset,
        },
        BinaryOp::Lt => IrOp::CmpInt {
            kind: CmpKind::Lt,
            offset,
        },
        BinaryOp::Lte => IrOp::CmpInt {
            kind: CmpKind::Lte,
            offset,
        },
        BinaryOp::Gt => IrOp::CmpInt {
            kind: CmpKind::Gt,
            offset,
        },
        BinaryOp::Gte => IrOp::CmpInt {
            kind: CmpKind::Gte,
            offset,
        },
        BinaryOp::Concat => IrOp::Concat { offset },
        BinaryOp::In => IrOp::In { offset },
        BinaryOp::NotIn => IrOp::NotIn { offset },
        BinaryOp::PlusPlus => IrOp::PlusPlus { offset },
        BinaryOp::MinusMinus => IrOp::MinusMinus { offset },
        BinaryOp::Range => IrOp::Range { offset },
        BinaryOp::StrictEq => IrOp::CmpInt {
            kind: CmpKind::StrictEq,
            offset,
        },
        BinaryOp::StrictBangEq => IrOp::CmpInt {
            kind: CmpKind::StrictNotEq,
            offset,
        },
        BinaryOp::BitwiseAnd => IrOp::BitwiseAnd { offset },
        BinaryOp::BitwiseOr => IrOp::BitwiseOr { offset },
        BinaryOp::BitwiseXor => IrOp::BitwiseXor { offset },
        BinaryOp::BitwiseShiftLeft => IrOp::BitwiseShiftLeft { offset },
        BinaryOp::BitwiseShiftRight => IrOp::BitwiseShiftRight { offset },
        BinaryOp::SteppedRange => IrOp::SteppedRange { offset },
        BinaryOp::Match | BinaryOp::And | BinaryOp::Or | BinaryOp::AndAnd | BinaryOp::OrOr => {
            unreachable!()
        }
    };
    ops.push(ir_op);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_try_expr(
    body: &Expr,
    rescue: &[crate::parser::Branch<crate::parser::Pattern>],
    catch: &[crate::parser::Branch<crate::parser::Pattern>],
    after: &Option<Box<Expr>>,
    offset: usize,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    let mut body_ops = Vec::new();
    lower_expr(body, current_module, struct_definitions, &mut body_ops)?;

    let rescue_branches = rescue
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

    let catch_branches = catch
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

    let after_ops = if let Some(after_expr) = after {
        let mut ops = Vec::new();
        lower_expr(after_expr, current_module, struct_definitions, &mut ops)?;
        Some(ops)
    } else {
        None
    };

    ops.push(IrOp::Try {
        body_ops,
        rescue_branches,
        catch_branches,
        after_ops,
        offset,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_for_expr(
    generators: &[crate::parser::ForGenerator],
    into: &Option<Box<Expr>>,
    reduce: &Option<Box<Expr>>,
    body: &Expr,
    offset: usize,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    let mut ir_generators = Vec::new();
    for generator in generators {
        let mut source_ops = Vec::new();
        lower_expr(
            generator.source(),
            current_module,
            struct_definitions,
            &mut source_ops,
        )?;

        let guard_ops = if let Some(guard) = generator.guard() {
            let mut guard_ops = Vec::new();
            lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
            Some(guard_ops)
        } else {
            None
        };

        ir_generators.push(IrForGenerator {
            pattern: lower_pattern(generator.pattern())?,
            source_ops,
            guard_ops,
        });
    }

    let mut body_ops = Vec::new();
    lower_expr(body, current_module, struct_definitions, &mut body_ops)?;

    let into_ops = match into {
        Some(into_expr) => {
            let mut ops = Vec::new();
            lower_expr(into_expr, current_module, struct_definitions, &mut ops)?;
            Some(ops)
        }
        None => None,
    };

    let reduce_ops = match reduce {
        Some(reduce_expr) => {
            let mut ops = Vec::new();
            lower_expr(reduce_expr, current_module, struct_definitions, &mut ops)?;
            Some(ops)
        }
        None => None,
    };

    ops.push(IrOp::For {
        generators: ir_generators,
        into_ops,
        reduce_ops,
        body_ops,
        offset,
    });
    Ok(())
}

pub(super) fn lower_collection_literals(
    expr: &Expr,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    match expr {
        Expr::Map {
            entries, offset, ..
        } => {
            if entries.is_empty() {
                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "map_empty".to_string(),
                    },
                    argc: 0,
                    offset: *offset,
                });
                return Ok(());
            }

            let first = &entries[0];
            lower_expr(first.key(), current_module, struct_definitions, ops)?;
            lower_expr(first.value(), current_module, struct_definitions, ops)?;

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "map".to_string(),
                },
                argc: 2,
                offset: *offset,
            });

            for entry in entries.iter().skip(1) {
                lower_expr(entry.key(), current_module, struct_definitions, ops)?;
                lower_expr(entry.value(), current_module, struct_definitions, ops)?;
                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "map_put".to_string(),
                    },
                    argc: 3,
                    offset: *offset,
                });
            }

            Ok(())
        }
        Expr::Struct {
            module,
            entries,
            offset,
            ..
        } => {
            let Some(struct_fields) = struct_definitions.get(module) else {
                return Err(LoweringError::unsupported("struct module", *offset));
            };

            ops.push(IrOp::ConstAtom {
                value: "__struct__".to_string(),
                offset: *offset,
            });
            ops.push(IrOp::ConstAtom {
                value: module.clone(),
                offset: *offset,
            });
            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "map".to_string(),
                },
                argc: 2,
                offset: *offset,
            });

            for (field_name, default) in struct_fields {
                ops.push(IrOp::ConstAtom {
                    value: field_name.clone(),
                    offset: *offset,
                });
                let field_value = entries
                    .iter()
                    .find_map(|entry| (entry.key == *field_name).then_some(&entry.value))
                    .unwrap_or(default);
                lower_expr(field_value, current_module, struct_definitions, ops)?;
                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "map_put".to_string(),
                    },
                    argc: 3,
                    offset: *offset,
                });
            }

            Ok(())
        }
        Expr::MapUpdate {
            base,
            updates,
            offset,
            ..
        } => {
            if updates.is_empty() {
                return Err(LoweringError::unsupported("map update arity", *offset));
            }

            lower_expr(base, current_module, struct_definitions, ops)?;

            for entry in updates {
                ops.push(IrOp::ConstAtom {
                    value: entry.key.clone(),
                    offset: *offset,
                });
                lower_expr(&entry.value, current_module, struct_definitions, ops)?;

                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "map_update".to_string(),
                    },
                    argc: 3,
                    offset: *offset,
                });
            }
            Ok(())
        }
        Expr::StructUpdate {
            base,
            updates,
            offset,
            ..
        } => {
            if updates.is_empty() {
                return Err(LoweringError::unsupported("struct update arity", *offset));
            }

            lower_expr(base, current_module, struct_definitions, ops)?;

            for entry in updates {
                ops.push(IrOp::ConstAtom {
                    value: entry.key.clone(),
                    offset: *offset,
                });
                lower_expr(&entry.value, current_module, struct_definitions, ops)?;

                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "map_update".to_string(),
                    },
                    argc: 3,
                    offset: *offset,
                });
            }
            Ok(())
        }
        Expr::Keyword {
            entries, offset, ..
        } => {
            if entries.is_empty() {
                return Err(LoweringError::unsupported("keyword literal arity", *offset));
            }

            let first = &entries[0];
            ops.push(IrOp::ConstAtom {
                value: first.key.clone(),
                offset: *offset,
            });
            lower_expr(&first.value, current_module, struct_definitions, ops)?;

            ops.push(IrOp::Call {
                callee: IrCallTarget::Builtin {
                    name: "keyword".to_string(),
                },
                argc: 2,
                offset: *offset,
            });

            for entry in entries.iter().skip(1) {
                ops.push(IrOp::ConstAtom {
                    value: entry.key.clone(),
                    offset: *offset,
                });
                lower_expr(&entry.value, current_module, struct_definitions, ops)?;

                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "keyword_append".to_string(),
                    },
                    argc: 3,
                    offset: *offset,
                });
            }

            Ok(())
        }
        _ => Ok(()),
    }
}
