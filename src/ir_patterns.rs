use super::*;

pub(super) fn lower_pipe_expr(
    left: &Expr,
    right: &Expr,
    pipe_offset: usize,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
) -> Result<(), LoweringError> {
    lower_expr(left, current_module, struct_definitions, ops)?;

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
        lower_expr(arg, current_module, struct_definitions, ops)?;
    }

    ops.push(IrOp::Call {
        callee: qualify_call_target(current_module, callee),
        argc: args.len() + 1,
        offset: *offset,
    });

    Ok(())
}

pub(super) fn lower_expr_pattern(expr: &Expr) -> Result<IrPattern, LoweringError> {
    match expr {
        Expr::Atom { value, .. } => Ok(IrPattern::Atom {
            value: value.clone(),
        }),
        Expr::Variable { name, .. } if name == "_" => Ok(IrPattern::Wildcard),
        Expr::Variable { name, .. } => Ok(IrPattern::Bind { name: name.clone() }),
        Expr::Int { value, .. } => Ok(IrPattern::Integer { value: *value }),
        Expr::Bool { value, .. } => Ok(IrPattern::Bool { value: *value }),
        Expr::Nil { .. } => Ok(IrPattern::Nil),
        Expr::String { value, .. } => Ok(IrPattern::String {
            value: value.clone(),
        }),
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
            Ok(IrPattern::List { items, tail: None })
        }
        Expr::Map { entries, .. } => {
            let entries = entries
                .iter()
                .map(|entry| {
                    Ok(IrMapPatternEntry {
                        key: lower_expr_pattern(entry.key())?,
                        value: lower_expr_pattern(entry.value())?,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;
            Ok(IrPattern::Map { entries })
        }
        Expr::Struct {
            module, entries, ..
        } => {
            let mut lowered_entries = vec![IrMapPatternEntry {
                key: IrPattern::Atom {
                    value: "__struct__".to_string(),
                },
                value: IrPattern::Atom {
                    value: module.clone(),
                },
            }];

            for entry in entries {
                lowered_entries.push(IrMapPatternEntry {
                    key: IrPattern::Atom {
                        value: entry.key.clone(),
                    },
                    value: lower_expr_pattern(&entry.value)?,
                });
            }

            Ok(IrPattern::Map {
                entries: lowered_entries,
            })
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

pub(super) fn lower_pattern(pattern: &Pattern) -> Result<IrPattern, LoweringError> {
    match pattern {
        Pattern::Atom { value } => Ok(IrPattern::Atom {
            value: value.clone(),
        }),
        Pattern::Bind { name } => Ok(IrPattern::Bind { name: name.clone() }),
        Pattern::Pin { name } => Ok(IrPattern::Pin { name: name.clone() }),
        Pattern::Wildcard => Ok(IrPattern::Wildcard),
        Pattern::Integer { value } => Ok(IrPattern::Integer { value: *value }),
        Pattern::Bool { value } => Ok(IrPattern::Bool { value: *value }),
        Pattern::Nil => Ok(IrPattern::Nil),
        Pattern::String { value } => Ok(IrPattern::String {
            value: value.clone(),
        }),
        Pattern::Tuple { items } => {
            let items = items
                .iter()
                .map(lower_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;

            Ok(IrPattern::Tuple { items })
        }
        Pattern::List { items, tail } => {
            let items = items
                .iter()
                .map(lower_pattern)
                .collect::<Result<Vec<_>, LoweringError>>()?;
            let tail = tail
                .as_ref()
                .map(|tail| lower_pattern(tail))
                .transpose()?
                .map(Box::new);

            Ok(IrPattern::List { items, tail })
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
        Pattern::Struct {
            module, entries, ..
        } => {
            let mut lowered_entries = vec![IrMapPatternEntry {
                key: IrPattern::Atom {
                    value: "__struct__".to_string(),
                },
                value: IrPattern::Atom {
                    value: module.clone(),
                },
            }];

            for entry in entries {
                lowered_entries.push(IrMapPatternEntry {
                    key: IrPattern::Atom {
                        value: entry.key.clone(),
                    },
                    value: lower_pattern(entry.value())?,
                });
            }

            Ok(IrPattern::Map {
                entries: lowered_entries,
            })
        }
        Pattern::Bitstring { items } => {
            let segments = items
                .iter()
                .map(|item| match item {
                    Pattern::Integer { value } => Ok(IrBitstringSegment::Literal {
                        value: *value as u8,
                    }),
                    Pattern::Bind { name } => Ok(IrBitstringSegment::Bind { name: name.clone() }),
                    Pattern::Wildcard => Ok(IrBitstringSegment::Wildcard),
                    _other => Err(LoweringError::unsupported("bitstring pattern segment", 0)),
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;
            Ok(IrPattern::Bitstring { segments })
        }
    }
}

pub(super) fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

pub(super) fn qualify_call_target(current_module: &str, callee: &str) -> IrCallTarget {
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

pub(super) fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err"
            | "tuple"
            | "list"
            | "map"
            | "keyword"
            | "protocol_dispatch"
            | "host_call"
            | "div"
            | "rem"
            | "byte_size"
            | "bit_size"
    ) || guard_builtins::is_guard_builtin(callee)
}
