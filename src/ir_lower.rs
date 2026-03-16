use super::*;

pub(super) fn lower_named_function(
    qualified_name: &str,
    current_module: &str,
    params: &[Parameter],
    guard: Option<&Expr>,
    body: &Expr,
    struct_definitions: &StructDefinitions,
    module_attrs: &HashMap<String, Expr>,
) -> Result<IrFunction, LoweringError> {
    let subst_body = substitute_module_attrs(body.clone(), module_attrs);
    let mut ops = Vec::new();
    lower_expr(&subst_body, current_module, struct_definitions, &mut ops)?;
    ops.push(IrOp::Return {
        offset: body.offset(),
    });

    let guard_ops = if let Some(guard) = guard {
        let subst_guard = substitute_module_attrs(guard.clone(), module_attrs);
        let mut guard_ops = Vec::new();
        lower_expr(
            &subst_guard,
            current_module,
            struct_definitions,
            &mut guard_ops,
        )?;
        Some(guard_ops)
    } else {
        None
    };

    let lowered_params = params
        .iter()
        .map(|param| param.name().to_string())
        .collect::<Vec<_>>();

    Ok(IrFunction {
        name: qualified_name.to_string(),
        param_patterns: lower_param_patterns(params, &lowered_params)?,
        params: lowered_params,
        guard_ops,
        ops,
    })
}

pub(super) fn lower_protocol_impl_function(
    protocol_impl: &ProtocolImplDecl,
    function: &ProtocolImplFunction,
    struct_definitions: &StructDefinitions,
) -> Result<IrFunction, LoweringError> {
    let qualified_name = protocol_impl_function_name(
        &protocol_impl.protocol,
        &protocol_impl.target,
        &function.name,
    );

    let empty_attrs = HashMap::new();
    lower_named_function(
        &qualified_name,
        &protocol_impl.module_name,
        &function.params,
        function.guard.as_ref(),
        &function.body,
        struct_definitions,
        &empty_attrs,
    )
}

pub(super) fn lower_protocol_dispatch_function(
    protocol: &ProtocolDecl,
    signature: &ProtocolFunctionSignature,
    dispatch_impls: &ProtocolDispatchImpls,
) -> Result<IrFunction, LoweringError> {
    if signature.params.is_empty() {
        return Err(LoweringError::unsupported("protocol dispatch arity", 0));
    }

    let dispatch_key = (protocol.name.clone(), signature.name.clone());
    let implementations = dispatch_impls
        .get(&dispatch_key)
        .cloned()
        .unwrap_or_default();

    let mut tuple_impl = None;
    let mut map_impl = None;
    let mut struct_impls = Vec::new();

    for (target, implementation) in implementations {
        match target.as_str() {
            "Tuple" => tuple_impl = Some(implementation),
            "Map" => map_impl = Some(implementation),
            _ => struct_impls.push((target, implementation)),
        }
    }

    let dispatch_param = signature
        .params
        .first()
        .expect("protocol dispatch should include first parameter")
        .clone();

    let mut top_level_branches = Vec::new();
    for (target, implementation) in struct_impls {
        top_level_branches.push(IrCaseBranch {
            pattern: IrPattern::Map {
                entries: vec![IrMapPatternEntry {
                    key: IrPattern::Atom {
                        value: "__struct__".to_string(),
                    },
                    value: IrPattern::Atom { value: target },
                }],
            },
            guard_ops: None,
            ops: build_protocol_impl_call_ops(&implementation, &signature.params, 0),
        });
    }

    top_level_branches.push(IrCaseBranch {
        pattern: IrPattern::Map {
            entries: vec![IrMapPatternEntry {
                key: IrPattern::Atom {
                    value: "__struct__".to_string(),
                },
                value: IrPattern::Bind {
                    name: "__tonic_struct_name".to_string(),
                },
            }],
        },
        guard_ops: None,
        ops: build_protocol_missing_impl_ops(&protocol.name, &signature.name, "struct", 0),
    });

    top_level_branches.push(IrCaseBranch {
        pattern: IrPattern::Wildcard,
        guard_ops: None,
        ops: build_non_struct_protocol_dispatch_ops(
            &protocol.name,
            &signature.name,
            &dispatch_param,
            &signature.params,
            tuple_impl.as_deref(),
            map_impl.as_deref(),
            0,
        ),
    });

    Ok(IrFunction {
        name: qualify_function_name(&protocol.name, &signature.name),
        params: signature.params.clone(),
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::LoadVariable {
                name: dispatch_param,
                offset: 0,
            },
            IrOp::Case {
                branches: top_level_branches,
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    })
}

pub(super) fn build_non_struct_protocol_dispatch_ops(
    protocol: &str,
    function: &str,
    dispatch_param: &str,
    params: &[String],
    tuple_impl: Option<&str>,
    map_impl: Option<&str>,
    offset: usize,
) -> Vec<IrOp> {
    let mut ops = vec![
        IrOp::LoadVariable {
            name: dispatch_param.to_string(),
            offset,
        },
        IrOp::Call {
            callee: IrCallTarget::Builtin {
                name: "protocol_dispatch".to_string(),
            },
            argc: 1,
            offset,
        },
    ];

    let tuple_ops = tuple_impl
        .map(|implementation| build_protocol_impl_call_ops(implementation, params, offset))
        .unwrap_or_else(|| build_protocol_missing_impl_ops(protocol, function, "tuple", offset));

    let map_ops = map_impl
        .map(|implementation| build_protocol_impl_call_ops(implementation, params, offset))
        .unwrap_or_else(|| build_protocol_missing_impl_ops(protocol, function, "map", offset));

    ops.push(IrOp::Case {
        branches: vec![
            IrCaseBranch {
                pattern: IrPattern::Integer { value: 1 },
                guard_ops: None,
                ops: tuple_ops,
            },
            IrCaseBranch {
                pattern: IrPattern::Integer { value: 2 },
                guard_ops: None,
                ops: map_ops,
            },
            IrCaseBranch {
                pattern: IrPattern::Wildcard,
                guard_ops: None,
                ops: build_protocol_missing_impl_ops(protocol, function, "value", offset),
            },
        ],
        offset,
    });

    ops
}

pub(super) fn build_protocol_impl_call_ops(
    implementation: &str,
    params: &[String],
    offset: usize,
) -> Vec<IrOp> {
    let mut ops = params
        .iter()
        .map(|param| IrOp::LoadVariable {
            name: param.clone(),
            offset,
        })
        .collect::<Vec<_>>();

    ops.push(IrOp::Call {
        callee: IrCallTarget::Function {
            name: implementation.to_string(),
        },
        argc: params.len(),
        offset,
    });

    ops
}

pub(super) fn build_protocol_missing_impl_ops(
    protocol: &str,
    function: &str,
    target: &str,
    offset: usize,
) -> Vec<IrOp> {
    vec![
        IrOp::ConstString {
            value: format!("protocol {protocol}.{function} has no implementation for {target}"),
            offset,
        },
        IrOp::Raise { offset },
    ]
}

pub(super) fn protocol_impl_function_name(protocol: &str, target: &str, function: &str) -> String {
    format!("__tonic_protocol_impl.{protocol}.{target}.{function}")
}

pub(super) fn lower_param_patterns(
    params: &[Parameter],
    lowered_params: &[String],
) -> Result<Option<Vec<IrPattern>>, LoweringError> {
    let lowered_patterns = params
        .iter()
        .map(|param| lower_pattern(param.pattern()))
        .collect::<Result<Vec<_>, _>>()?;

    let is_simple_bind_shape = lowered_patterns.iter().zip(lowered_params.iter()).all(
        |(pattern, param_name)| matches!(pattern, IrPattern::Bind { name } if name == param_name),
    );

    if is_simple_bind_shape {
        Ok(None)
    } else {
        Ok(Some(lowered_patterns))
    }
}

pub(super) fn lower_default_argument_wrappers(
    module_name: &str,
    function: &crate::parser::Function,
    struct_definitions: &StructDefinitions,
    module_attrs: &HashMap<String, Expr>,
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
            let subst_default = substitute_module_attrs(default.clone(), module_attrs);
            lower_expr(&subst_default, module_name, struct_definitions, &mut ops)?;
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
