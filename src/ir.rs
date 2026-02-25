use crate::guard_builtins;
use crate::parser::{
    Ast, BinaryOp, Expr, ModuleForm, Parameter, Pattern, ProtocolFunctionSignature,
    ProtocolImplFunction,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    ConstFloat {
        value: String,
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
    ToString {
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
    Try {
        body_ops: Vec<IrOp>,
        rescue_branches: Vec<IrCaseBranch>,
        catch_branches: Vec<IrCaseBranch>,
        after_ops: Option<Vec<IrOp>>,
        offset: usize,
    },
    Raise {
        offset: usize,
    },
    For {
        generators: Vec<(IrPattern, Vec<IrOp>)>,
        into_ops: Option<Vec<IrOp>>,
        body_ops: Vec<IrOp>,
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
    Atom {
        value: String,
    },
    Bind {
        name: String,
    },
    Pin {
        name: String,
    },
    Wildcard,
    Integer {
        value: i64,
    },
    Bool {
        value: bool,
    },
    Nil,
    String {
        value: String,
    },
    Tuple {
        items: Vec<IrPattern>,
    },
    List {
        items: Vec<IrPattern>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tail: Option<Box<IrPattern>>,
    },
    Map {
        entries: Vec<IrMapPatternEntry>,
    },
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

type StructDefinitions = HashMap<String, Vec<(String, Expr)>>;

type ProtocolDispatchImpls = HashMap<(String, String), Vec<(String, String)>>;

#[derive(Debug, Clone)]
struct ProtocolDecl {
    name: String,
    functions: Vec<ProtocolFunctionSignature>,
}

#[derive(Debug, Clone)]
struct ProtocolImplDecl {
    module_name: String,
    protocol: String,
    target: String,
    functions: Vec<ProtocolImplFunction>,
}

pub fn lower_ast_to_ir(ast: &Ast) -> Result<IrProgram, LoweringError> {
    let mut functions = Vec::new();
    let struct_definitions = collect_struct_definitions(ast);
    let (protocol_decls, protocol_impls) = collect_protocol_forms(ast);

    for module in &ast.modules {
        for function in &module.functions {
            let lowered = lower_named_function(
                &qualify_function_name(&module.name, &function.name),
                module.name.as_str(),
                &function.params,
                function.guard(),
                &function.body,
                &struct_definitions,
            )?;
            functions.push(lowered);

            let wrappers = lower_default_argument_wrappers(
                module.name.as_str(),
                function,
                &struct_definitions,
            )?;
            functions.extend(wrappers);
        }
    }

    let mut dispatch_impls: ProtocolDispatchImpls = HashMap::new();
    for protocol_impl in &protocol_impls {
        for function in &protocol_impl.functions {
            let lowered =
                lower_protocol_impl_function(protocol_impl, function, &struct_definitions)?;
            dispatch_impls
                .entry((protocol_impl.protocol.clone(), function.name.clone()))
                .or_default()
                .push((protocol_impl.target.clone(), lowered.name.clone()));
            functions.push(lowered);
        }
    }

    for protocol in &protocol_decls {
        for signature in &protocol.functions {
            functions.push(lower_protocol_dispatch_function(
                protocol,
                signature,
                &dispatch_impls,
            )?);
        }
    }

    Ok(IrProgram { functions })
}

fn collect_struct_definitions(ast: &Ast) -> StructDefinitions {
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

fn collect_protocol_forms(ast: &Ast) -> (Vec<ProtocolDecl>, Vec<ProtocolImplDecl>) {
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

fn lower_named_function(
    qualified_name: &str,
    current_module: &str,
    params: &[Parameter],
    guard: Option<&Expr>,
    body: &Expr,
    struct_definitions: &StructDefinitions,
) -> Result<IrFunction, LoweringError> {
    let mut ops = Vec::new();
    lower_expr(body, current_module, struct_definitions, &mut ops)?;
    ops.push(IrOp::Return {
        offset: body.offset(),
    });

    let guard_ops = if let Some(guard) = guard {
        let mut guard_ops = Vec::new();
        lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
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

fn lower_protocol_impl_function(
    protocol_impl: &ProtocolImplDecl,
    function: &ProtocolImplFunction,
    struct_definitions: &StructDefinitions,
) -> Result<IrFunction, LoweringError> {
    let qualified_name = protocol_impl_function_name(
        &protocol_impl.protocol,
        &protocol_impl.target,
        &function.name,
    );

    lower_named_function(
        &qualified_name,
        &protocol_impl.module_name,
        &function.params,
        function.guard.as_ref(),
        &function.body,
        struct_definitions,
    )
}

fn lower_protocol_dispatch_function(
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

fn build_non_struct_protocol_dispatch_ops(
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

fn build_protocol_impl_call_ops(
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

fn build_protocol_missing_impl_ops(
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

fn protocol_impl_function_name(protocol: &str, target: &str, function: &str) -> String {
    format!("__tonic_protocol_impl.{protocol}.{target}.{function}")
}

fn lower_param_patterns(
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

fn lower_default_argument_wrappers(
    module_name: &str,
    function: &crate::parser::Function,
    struct_definitions: &StructDefinitions,
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
            lower_expr(default, module_name, struct_definitions, &mut ops)?;
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

fn lower_expr(
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
            }
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
                lower_expr(right, current_module, struct_definitions, ops)?;
                let pattern = lower_expr_pattern(left)?;
                ops.push(IrOp::Match {
                    pattern,
                    offset: *offset,
                });
                return Ok(());
            }

            lower_expr(left, current_module, struct_definitions, ops)?;

            match op {
                BinaryOp::AndAnd => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
                    ops.push(IrOp::AndAnd {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::OrOr => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
                    ops.push(IrOp::OrOr {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::And => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
                    ops.push(IrOp::And {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                BinaryOp::Or => {
                    let mut right_ops = Vec::new();
                    lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
                    ops.push(IrOp::Or {
                        right_ops,
                        offset: *offset,
                    });
                    return Ok(());
                }
                _ => {}
            }

            lower_expr(right, current_module, struct_definitions, ops)?;
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
        } => {
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
                offset: *offset,
            });
            Ok(())
        }
        Expr::Raise { error, offset, .. } => {
            lower_expr(error, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Raise { offset: *offset });
            Ok(())
        }
        Expr::For {
            generators,
            into,
            body,
            offset,
            ..
        } => {
            let mut ir_generators = Vec::new();
            for (pattern, generator) in generators {
                let mut gen_ops = Vec::new();
                lower_expr(generator, current_module, struct_definitions, &mut gen_ops)?;
                ir_generators.push((lower_pattern(pattern)?, gen_ops));
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

            ops.push(IrOp::For {
                generators: ir_generators,
                into_ops,
                body_ops,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Group { inner, .. } => lower_expr(inner, current_module, struct_definitions, ops),
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

fn lower_expr_pattern(expr: &Expr) -> Result<IrPattern, LoweringError> {
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

fn lower_pattern(pattern: &Pattern) -> Result<IrPattern, LoweringError> {
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
    ) || guard_builtins::is_guard_builtin(callee)
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
    fn lower_ast_supports_for_comprehension_ops() {
        let source = "defmodule Demo do\n  def run() do\n    for x <- list(1, 2) do\n      x + 1\n    end\n  end\nend\n";
        let tokens =
            scan_tokens(source).expect("scanner should tokenize for comprehension fixture");
        let ast = parse_ast(&tokens).expect("parser should build for comprehension fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should support for comprehensions");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        assert_eq!(
            json["functions"][0]["ops"],
            serde_json::json!([
                {
                    "op":"for",
                    "into_ops": null,
                    "generators":[
                        [
                            {"kind":"bind","name":"x"},
                            [
                                {"op":"const_int","value":1,"offset":51},
                                {"op":"const_int","value":2,"offset":54},
                                {"op":"call","callee":{"kind":"builtin","name":"list"},"argc":2,"offset":46}
                            ]
                        ]
                    ],
                    "body_ops":[
                        {"op":"load_variable","name":"x","offset":66},
                        {"op":"const_int","value":1,"offset":70},
                        {"op":"add_int","offset":66}
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

    #[test]
    fn lower_ast_generates_protocol_dispatcher_and_impl_functions() {
        let source = "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  def run() do\n    Size.size(tuple(1, 2))\n  end\nend\n";
        let tokens =
            scan_tokens(source).expect("scanner should tokenize protocol lowering fixture");
        let ast = parse_ast(&tokens).expect("parser should build protocol lowering fixture ast");

        let ir = lower_ast_to_ir(&ast).expect("lowering should support protocol forms");
        let json = serde_json::to_value(&ir).expect("ir should serialize");

        let names = json["functions"]
            .as_array()
            .expect("lowered functions should be an array")
            .iter()
            .map(|function| {
                function["name"]
                    .as_str()
                    .expect("lowered function should include a name")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert!(names.iter().any(|name| name == "Demo.run"));
        assert!(names.iter().any(|name| name == "Size.size"));
        assert!(names
            .iter()
            .any(|name| name == "__tonic_protocol_impl.Size.Tuple.size"));

        let size_function = json["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["name"] == "Size.size")
            .expect("lowered ir should include protocol dispatcher function");

        let serialized_ops = serde_json::to_string(&size_function["ops"])
            .expect("protocol dispatcher ops should serialize");
        assert!(serialized_ops.contains("protocol_dispatch"));
    }
}
