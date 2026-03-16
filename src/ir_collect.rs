use super::*;

pub(super) fn collect_struct_definitions(ast: &Ast) -> StructDefinitions {
    let mut definitions = HashMap::new();

    for module in &ast.modules {
        if let Some(fields) = module.forms.iter().find_map(|form| {
            if let ModuleForm::Defstruct { fields } = form {
                Some(
                    fields
                        .iter()
                        .map(|field| (field.name.clone(), field.default.clone()))
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        }) {
            definitions.insert(module.name.clone(), fields);
        }
    }

    definitions
}

pub(super) fn collect_protocol_forms(ast: &Ast) -> (Vec<ProtocolDecl>, Vec<ProtocolImplDecl>) {
    let mut protocols = Vec::new();
    let mut implementations = Vec::new();

    for module in &ast.modules {
        for form in &module.forms {
            match form {
                ModuleForm::Defprotocol { name, functions } => {
                    protocols.push(ProtocolDecl {
                        name: name.clone(),
                        functions: functions.clone(),
                    });
                }
                ModuleForm::Defimpl {
                    protocol,
                    target,
                    functions,
                } => {
                    implementations.push(ProtocolImplDecl {
                        module_name: module.name.clone(),
                        protocol: protocol.clone(),
                        target: target.clone(),
                        functions: functions.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    (protocols, implementations)
}

/// Substitute module attribute references (@attr_name as Variable expressions)
/// with the attribute's value expression. This enables @attr in function bodies.
pub(super) fn substitute_module_attrs(expr: Expr, attrs: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Variable { ref name, .. } if name.starts_with('@') => {
            let attr_name = &name[1..];
            if let Some(value) = attrs.get(attr_name) {
                return value.clone();
            }
            expr
        }
        Expr::Tuple { id, offset, items } => Expr::Tuple {
            id,
            offset,
            items: items
                .into_iter()
                .map(|e| substitute_module_attrs(e, attrs))
                .collect(),
        },
        Expr::List { id, offset, items } => Expr::List {
            id,
            offset,
            items: items
                .into_iter()
                .map(|e| substitute_module_attrs(e, attrs))
                .collect(),
        },
        Expr::Binary {
            id,
            offset,
            op,
            left,
            right,
        } => Expr::Binary {
            id,
            offset,
            op,
            left: Box::new(substitute_module_attrs(*left, attrs)),
            right: Box::new(substitute_module_attrs(*right, attrs)),
        },
        Expr::Unary {
            id,
            offset,
            op,
            value,
        } => Expr::Unary {
            id,
            offset,
            op,
            value: Box::new(substitute_module_attrs(*value, attrs)),
        },
        Expr::Call {
            id,
            offset,
            callee,
            args,
        } => Expr::Call {
            id,
            offset,
            callee,
            args: args
                .into_iter()
                .map(|e| substitute_module_attrs(e, attrs))
                .collect(),
        },
        Expr::FieldAccess {
            id,
            offset,
            base,
            label,
        } => Expr::FieldAccess {
            id,
            offset,
            label,
            base: Box::new(substitute_module_attrs(*base, attrs)),
        },
        Expr::IndexAccess {
            id,
            offset,
            base,
            index,
        } => Expr::IndexAccess {
            id,
            offset,
            base: Box::new(substitute_module_attrs(*base, attrs)),
            index: Box::new(substitute_module_attrs(*index, attrs)),
        },
        Expr::Pipe {
            id,
            offset,
            left,
            right,
        } => Expr::Pipe {
            id,
            offset,
            left: Box::new(substitute_module_attrs(*left, attrs)),
            right: Box::new(substitute_module_attrs(*right, attrs)),
        },
        Expr::Group { id, offset, inner } => Expr::Group {
            id,
            offset,
            inner: Box::new(substitute_module_attrs(*inner, attrs)),
        },
        Expr::Question { id, offset, value } => Expr::Question {
            id,
            offset,
            value: Box::new(substitute_module_attrs(*value, attrs)),
        },
        // For other expressions (Case, For, Try, If/Unless via Call, etc.), just return as-is.
        // The simple attribute value case (e.g. `@my_value` as the direct body) is handled above.
        other => other,
    }
}
