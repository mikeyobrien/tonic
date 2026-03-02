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
pub(crate) struct IrForGenerator {
    pub(crate) pattern: IrPattern,
    pub(crate) source_ops: Vec<IrOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) guard_ops: Option<Vec<IrOp>>,
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
        generators: Vec<IrForGenerator>,
        into_ops: Option<Vec<IrOp>>,
        reduce_ops: Option<Vec<IrOp>>,
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
    NotIn {
        offset: usize,
    },
    BitwiseAnd {
        offset: usize,
    },
    BitwiseOr {
        offset: usize,
    },
    BitwiseXor {
        offset: usize,
    },
    BitwiseNot {
        offset: usize,
    },
    BitwiseShiftLeft {
        offset: usize,
    },
    BitwiseShiftRight {
        offset: usize,
    },
    SteppedRange {
        offset: usize,
    },
    /// Construct a bitstring/binary value from a list of byte elements already on the stack.
    /// `count` is the number of elements to pop. The result is a `Binary(Vec<u8>)` runtime value.
    Bitstring {
        count: usize,
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
    StrictEq,
    StrictNotEq,
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
    /// Pattern that matches a binary/bitstring value.
    /// Each segment is matched by binding or literal integer byte value.
    Bitstring {
        segments: Vec<IrBitstringSegment>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IrMapPatternEntry {
    pub(crate) key: IrPattern,
    pub(crate) value: IrPattern,
}

/// A single segment within a bitstring pattern or literal.
/// For simplicity, each segment represents one byte-wide element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum IrBitstringSegment {
    /// Literal integer byte value (e.g. `<<72, ...>>`)
    Literal { value: u8 },
    /// Bind to a variable name
    Bind { name: String },
    /// Wildcard (match any byte)
    Wildcard,
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
        // Build module attribute map for @attr substitution in function bodies
        let module_attrs: HashMap<String, Expr> = module
            .attributes
            .iter()
            .map(|attr| (attr.name.clone(), attr.value.clone()))
            .collect();

        for function in &module.functions {
            let lowered = lower_named_function(
                &qualify_function_name(&module.name, &function.name),
                module.name.as_str(),
                &function.params,
                function.guard(),
                &function.body,
                &struct_definitions,
                &module_attrs,
            )?;
            functions.push(lowered);

            let wrappers = lower_default_argument_wrappers(
                module.name.as_str(),
                function,
                &struct_definitions,
                &module_attrs,
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

/// Substitute module attribute references (@attr_name as Variable expressions)
/// with the attribute's value expression. This enables @attr in function bodies.
fn substitute_module_attrs(expr: Expr, attrs: &HashMap<String, Expr>) -> Expr {
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
            items: items.into_iter().map(|e| substitute_module_attrs(e, attrs)).collect(),
        },
        Expr::List { id, offset, items } => Expr::List {
            id,
            offset,
            items: items.into_iter().map(|e| substitute_module_attrs(e, attrs)).collect(),
        },
        Expr::Binary { id, offset, op, left, right } => Expr::Binary {
            id, offset, op,
            left: Box::new(substitute_module_attrs(*left, attrs)),
            right: Box::new(substitute_module_attrs(*right, attrs)),
        },
        Expr::Unary { id, offset, op, value } => Expr::Unary {
            id, offset, op,
            value: Box::new(substitute_module_attrs(*value, attrs)),
        },
        Expr::Call { id, offset, callee, args } => Expr::Call {
            id, offset, callee,
            args: args.into_iter().map(|e| substitute_module_attrs(e, attrs)).collect(),
        },
        Expr::FieldAccess { id, offset, base, label } => Expr::FieldAccess {
            id, offset, label,
            base: Box::new(substitute_module_attrs(*base, attrs)),
        },
        Expr::IndexAccess { id, offset, base, index } => Expr::IndexAccess {
            id, offset,
            base: Box::new(substitute_module_attrs(*base, attrs)),
            index: Box::new(substitute_module_attrs(*index, attrs)),
        },
        Expr::Pipe { id, offset, left, right } => Expr::Pipe {
            id, offset,
            left: Box::new(substitute_module_attrs(*left, attrs)),
            right: Box::new(substitute_module_attrs(*right, attrs)),
        },
        Expr::Group { id, offset, inner } => Expr::Group {
            id, offset,
            inner: Box::new(substitute_module_attrs(*inner, attrs)),
        },
        Expr::Question { id, offset, value } => Expr::Question {
            id, offset,
            value: Box::new(substitute_module_attrs(*value, attrs)),
        },
        // For other expressions (Case, For, Try, If/Unless via Call, etc.), just return as-is.
        // The simple attribute value case (e.g. `@my_value` as the direct body) is handled above.
        other => other,
    }
}

fn lower_named_function(
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
        lower_expr(&subst_guard, current_module, struct_definitions, &mut guard_ops)?;
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
            lower_expr(base, current_module, struct_definitions, ops)?;

            for update in updates {
                lower_expr(update.key(), current_module, struct_definitions, ops)?;
                lower_expr(update.value(), current_module, struct_definitions, ops)?;
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
                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin {
                        name: "keyword_empty".to_string(),
                    },
                    argc: 0,
                    offset: *offset,
                });
                return Ok(());
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
        Expr::Atom { value, offset, .. } => {
            ops.push(IrOp::ConstAtom {
                value: value.clone(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Variable { name, offset, .. } => {
            ops.push(IrOp::LoadVariable {
                name: name.clone(),
                offset: *offset,
            });
            Ok(())
        }
        Expr::Block { exprs, offset, .. } => {
            if exprs.is_empty() {
                ops.push(IrOp::ConstNil { offset: *offset });
                return Ok(());
            }

            for expr in exprs {
                lower_expr(expr, current_module, struct_definitions, ops)?;
            }

            Ok(())
        }
        Expr::Assign { name, value, offset, .. } => {
            lower_expr(value, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Match {
                pattern: IrPattern::Bind { name: name.clone() },
                offset: *offset,
            });
            Ok(())
        }
        Expr::Match { left, right, offset, .. } => {
            lower_expr(right, current_module, struct_definitions, ops)?;
            let pattern = lower_pattern(left)?;
            ops.push(IrOp::Match {
                pattern,
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
            lower_call(callee, args, current_module, struct_definitions, ops, *offset)
        }
        Expr::Pipe { left, right, offset, .. } => {
            lower_pipe(left, right, current_module, struct_definitions, ops, *offset)
        }
        Expr::Binary { op, left, right, offset, .. } => {
            lower_binary(op, left, right, current_module, struct_definitions, ops, *offset)
        }
        Expr::Unary { op, value, offset, .. } => {
            lower_unary(op, value, current_module, struct_definitions, ops, *offset)
        }
        Expr::Group { inner, .. } => {
            lower_expr(inner, current_module, struct_definitions, ops)
        }
        Expr::Case {
            subject,
            branches,
            offset,
            ..
        } => {
            lower_expr(subject, current_module, struct_definitions, ops)?;

            let mut ir_branches = Vec::new();
            for branch in branches {
                let pattern = lower_pattern(&branch.pattern)?;

                let guard_ops = if let Some(guard) = &branch.guard {
                    let mut guard_ops = Vec::new();
                    lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
                    Some(guard_ops)
                } else {
                    None
                };

                let mut branch_ops = Vec::new();
                lower_expr(&branch.body, current_module, struct_definitions, &mut branch_ops)?;

                ir_branches.push(IrCaseBranch {
                    pattern,
                    guard_ops,
                    ops: branch_ops,
                });
            }

            ops.push(IrOp::Case {
                branches: ir_branches,
                offset: *offset,
            });

            Ok(())
        }
        Expr::Cond { branches, offset, .. } => {
            lower_cond(branches, current_module, struct_definitions, ops, *offset)
        }
        Expr::With {
            clauses,
            body,
            else_branches,
            offset,
            ..
        } => lower_with(
            clauses,
            body,
            else_branches.as_deref(),
            current_module,
            struct_definitions,
            ops,
            *offset,
        ),
        Expr::For {
            generators,
            into,
            reduce,
            body,
            offset,
            ..
        } => {
            let ir_generators = generators
                .iter()
                .map(|generator| {
                    let pattern = lower_pattern(&generator.pattern)?;
                    let mut source_ops = Vec::new();
                    lower_expr(
                        &generator.source,
                        current_module,
                        struct_definitions,
                        &mut source_ops,
                    )?;
                    let guard_ops = if let Some(guard) = &generator.guard {
                        let mut guard_ops = Vec::new();
                        lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
                        Some(guard_ops)
                    } else {
                        None
                    };
                    Ok(IrForGenerator {
                        pattern,
                        source_ops,
                        guard_ops,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            let into_ops = if let Some(into_expr) = into {
                let mut into_ops = Vec::new();
                lower_expr(into_expr, current_module, struct_definitions, &mut into_ops)?;
                Some(into_ops)
            } else {
                None
            };

            let reduce_ops = if let Some(reduce_expr) = reduce {
                let mut reduce_ops = Vec::new();
                lower_expr(reduce_expr, current_module, struct_definitions, &mut reduce_ops)?;
                Some(reduce_ops)
            } else {
                None
            };

            let mut body_ops = Vec::new();
            lower_expr(body, current_module, struct_definitions, &mut body_ops)?;

            ops.push(IrOp::For {
                generators: ir_generators,
                into_ops,
                reduce_ops,
                body_ops,
                offset: *offset,
            });

            Ok(())
        }
        Expr::Try {
            body,
            rescue_branches,
            catch_branches,
            after,
            offset,
            ..
        } => {
            let mut body_ops = Vec::new();
            lower_expr(body, current_module, struct_definitions, &mut body_ops)?;

            let rescue_ir_branches = rescue_branches
                .iter()
                .map(|branch| {
                    let pattern = lower_pattern(&branch.pattern)?;
                    let guard_ops = if let Some(guard) = &branch.guard {
                        let mut guard_ops = Vec::new();
                        lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
                        Some(guard_ops)
                    } else {
                        None
                    };
                    let mut branch_ops = Vec::new();
                    lower_expr(&branch.body, current_module, struct_definitions, &mut branch_ops)?;
                    Ok(IrCaseBranch {
                        pattern,
                        guard_ops,
                        ops: branch_ops,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            let catch_ir_branches = catch_branches
                .iter()
                .map(|branch| {
                    let pattern = lower_pattern(&branch.pattern)?;
                    let guard_ops = if let Some(guard) = &branch.guard {
                        let mut guard_ops = Vec::new();
                        lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
                        Some(guard_ops)
                    } else {
                        None
                    };
                    let mut branch_ops = Vec::new();
                    lower_expr(&branch.body, current_module, struct_definitions, &mut branch_ops)?;
                    Ok(IrCaseBranch {
                        pattern,
                        guard_ops,
                        ops: branch_ops,
                    })
                })
                .collect::<Result<Vec<_>, LoweringError>>()?;

            let after_ops = if let Some(after_expr) = after {
                let mut after_ops = Vec::new();
                lower_expr(after_expr, current_module, struct_definitions, &mut after_ops)?;
                Some(after_ops)
            } else {
                None
            };

            ops.push(IrOp::Try {
                body_ops,
                rescue_branches: rescue_ir_branches,
                catch_branches: catch_ir_branches,
                after_ops,
                offset: *offset,
            });

            Ok(())
        }
        Expr::Raise { value, offset, .. } => {
            lower_expr(value, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Raise { offset: *offset });
            Ok(())
        }
        Expr::Question { value, offset, .. } => {
            lower_expr(value, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Question { offset: *offset });
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
        Expr::Fn {
            params,
            body,
            offset,
            ..
        } => {
            let param_names: Vec<String> =
                params.iter().map(|p| p.name().to_string()).collect();
            let mut closure_ops = Vec::new();
            lower_expr(body, current_module, struct_definitions, &mut closure_ops)?;
            closure_ops.push(IrOp::Return { offset: *offset });

            ops.push(IrOp::MakeClosure {
                params: param_names,
                ops: closure_ops,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Capture {
            module: _,
            function,
            arity,
            offset,
            ..
        } => {
            let arity_val = *arity;
            let param_names: Vec<String> =
                (0..arity_val).map(|i| format!("__cap_arg{i}")).collect();
            let mut closure_ops: Vec<IrOp> = param_names
                .iter()
                .map(|p| IrOp::LoadVariable {
                    name: p.clone(),
                    offset: *offset,
                })
                .collect();

            let qualified = format!("{current_module}.{function}");
            closure_ops.push(IrOp::Call {
                callee: IrCallTarget::Function { name: qualified },
                argc: arity_val,
                offset: *offset,
            });
            closure_ops.push(IrOp::Return { offset: *offset });

            ops.push(IrOp::MakeClosure {
                params: param_names,
                ops: closure_ops,
                offset: *offset,
            });
            Ok(())
        }
        Expr::Bitstring { elements, offset, .. } => {
            let count = elements.len();
            for element in elements {
                lower_expr(&element.value, current_module, struct_definitions, ops)?;
            }
            ops.push(IrOp::Bitstring {
                count,
                offset: *offset,
            });
            Ok(())
        }
    }
}

fn lower_cond(
    branches: &[crate::parser::CondBranch],
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    if branches.is_empty() {
        ops.push(IrOp::ConstNil { offset });
        return Ok(());
    }

    // Build nested if-else from the branches, working backwards
    let last = branches.last().unwrap();
    let mut else_ops: Vec<IrOp> = Vec::new();
    lower_expr(&last.body, current_module, struct_definitions, &mut else_ops)?;

    for branch in branches[..branches.len() - 1].iter().rev() {
        let mut cond_ops = Vec::new();
        lower_expr(&branch.condition, current_module, struct_definitions, &mut cond_ops)?;

        let mut then_ops = Vec::new();
        lower_expr(&branch.body, current_module, struct_definitions, &mut then_ops)?;

        let true_branch = IrCaseBranch {
            pattern: IrPattern::Bool { value: true },
            guard_ops: None,
            ops: then_ops,
        };
        let false_branch = IrCaseBranch {
            pattern: IrPattern::Wildcard,
            guard_ops: None,
            ops: else_ops,
        };

        else_ops = cond_ops;
        else_ops.push(IrOp::Case {
            branches: vec![true_branch, false_branch],
            offset,
        });
    }

    ops.extend(else_ops);
    Ok(())
}

fn lower_with(
    clauses: &[crate::parser::WithClause],
    body: &Expr,
    else_branches: Option<&[crate::parser::CaseBranch]>,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    // Build the success path as a chain of match ops + body
    // Each `with pattern <- expr` clause is lowered as:
    //   1. Lower source expr
    //   2. Case on result: if pattern matches -> continue, else -> else branches or propagate
    //
    // We build from innermost (body) outwards

    let mut inner_ops: Vec<IrOp> = Vec::new();
    lower_expr(body, current_module, struct_definitions, &mut inner_ops)?;

    // Build else branch operations (or a raise if none)
    let else_ir_branches: Vec<IrCaseBranch> = if let Some(branches) = else_branches {
        branches
            .iter()
            .map(|branch| {
                let pattern = lower_pattern(&branch.pattern)?;
                let guard_ops = if let Some(guard) = &branch.guard {
                    let mut guard_ops = Vec::new();
                    lower_expr(guard, current_module, struct_definitions, &mut guard_ops)?;
                    Some(guard_ops)
                } else {
                    None
                };
                let mut branch_ops = Vec::new();
                lower_expr(&branch.body, current_module, struct_definitions, &mut branch_ops)?;
                Ok(IrCaseBranch {
                    pattern,
                    guard_ops,
                    ops: branch_ops,
                })
            })
            .collect::<Result<Vec<_>, LoweringError>>()?
    } else {
        vec![]
    };

    // Process clauses in reverse so that outer clauses wrap inner ones
    for clause in clauses.iter().rev() {
        let pattern = lower_pattern(&clause.pattern)?;

        let mut source_ops: Vec<IrOp> = Vec::new();
        lower_expr(&clause.source, current_module, struct_definitions, &mut source_ops)?;

        let success_branch = IrCaseBranch {
            pattern,
            guard_ops: None,
            ops: inner_ops,
        };

        let mut branches = vec![success_branch];
        if !else_ir_branches.is_empty() {
            branches.extend(else_ir_branches.clone());
        } else {
            // No else: wildcard that raises
            branches.push(IrCaseBranch {
                pattern: IrPattern::Bind {
                    name: "__with_no_match".to_string(),
                },
                guard_ops: None,
                ops: vec![
                    IrOp::LoadVariable {
                        name: "__with_no_match".to_string(),
                        offset,
                    },
                    IrOp::Raise { offset },
                ],
            });
        }

        inner_ops = source_ops;
        inner_ops.push(IrOp::Case { branches, offset });
    }

    ops.extend(inner_ops);
    Ok(())
}

fn lower_call(
    callee: &Expr,
    args: &[Expr],
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    match callee {
        Expr::Variable { name, .. } => {
            match name.as_str() {
                "if" => return lower_if(args, current_module, struct_definitions, ops, offset),
                "unless" => {
                    return lower_unless(args, current_module, struct_definitions, ops, offset)
                }
                _ => {}
            }

            if guard_builtins::is_guard_builtin(name) {
                for arg in args {
                    lower_expr(arg, current_module, struct_definitions, ops)?;
                }
                ops.push(IrOp::Call {
                    callee: IrCallTarget::Builtin { name: name.clone() },
                    argc: args.len(),
                    offset,
                });
                return Ok(());
            }

            let call_target = if name.contains('.') {
                IrCallTarget::Function { name: name.clone() }
            } else {
                IrCallTarget::Function {
                    name: qualify_function_name(current_module, name),
                }
            };

            for arg in args {
                lower_expr(arg, current_module, struct_definitions, ops)?;
            }

            ops.push(IrOp::Call {
                callee: call_target,
                argc: args.len(),
                offset,
            });
            Ok(())
        }
        Expr::FieldAccess { base, label, .. } => {
            // Module-qualified call: Module.function(args)
            let module_name = match base.as_ref() {
                Expr::Variable { name, .. } => name.clone(),
                _ => return Err(LoweringError::unsupported("complex module call", offset)),
            };

            let qualified_name = format!("{module_name}.{label}");

            for arg in args {
                lower_expr(arg, current_module, struct_definitions, ops)?;
            }

            ops.push(IrOp::Call {
                callee: IrCallTarget::Function {
                    name: qualified_name,
                },
                argc: args.len(),
                offset,
            });
            Ok(())
        }
        other => {
            // Dynamic function call: evaluate callee to get closure, then call it
            lower_expr(other, current_module, struct_definitions, ops)?;
            for arg in args {
                lower_expr(arg, current_module, struct_definitions, ops)?;
            }
            ops.push(IrOp::CallValue {
                argc: args.len(),
                offset,
            });
            Ok(())
        }
    }
}

fn lower_if(
    args: &[Expr],
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    let condition = args
        .first()
        .ok_or_else(|| LoweringError::unsupported("if condition", offset))?;
    let then_body = args
        .get(1)
        .ok_or_else(|| LoweringError::unsupported("if then body", offset))?;
    let else_body = args.get(2);

    lower_expr(condition, current_module, struct_definitions, ops)?;

    let mut then_ops = Vec::new();
    lower_expr(then_body, current_module, struct_definitions, &mut then_ops)?;

    let mut else_ops = Vec::new();
    if let Some(else_body) = else_body {
        lower_expr(else_body, current_module, struct_definitions, &mut else_ops)?;
    } else {
        else_ops.push(IrOp::ConstNil { offset });
    }

    ops.push(IrOp::Case {
        branches: vec![
            IrCaseBranch {
                pattern: IrPattern::Bool { value: true },
                guard_ops: None,
                ops: then_ops,
            },
            IrCaseBranch {
                pattern: IrPattern::Wildcard,
                guard_ops: None,
                ops: else_ops,
            },
        ],
        offset,
    });

    Ok(())
}

fn lower_unless(
    args: &[Expr],
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    let condition = args
        .first()
        .ok_or_else(|| LoweringError::unsupported("unless condition", offset))?;
    let then_body = args
        .get(1)
        .ok_or_else(|| LoweringError::unsupported("unless then body", offset))?;
    let else_body = args.get(2);

    lower_expr(condition, current_module, struct_definitions, ops)?;

    let mut then_ops = Vec::new();
    lower_expr(then_body, current_module, struct_definitions, &mut then_ops)?;

    let mut else_ops = Vec::new();
    if let Some(else_body) = else_body {
        lower_expr(else_body, current_module, struct_definitions, &mut else_ops)?;
    } else {
        else_ops.push(IrOp::ConstNil { offset });
    }

    // unless is: if NOT condition then then_body else else_body
    // Lowered as: case condition { false | nil -> then_body, _ -> else_body }
    ops.push(IrOp::Case {
        branches: vec![
            IrCaseBranch {
                pattern: IrPattern::Bool { value: false },
                guard_ops: None,
                ops: then_ops.clone(),
            },
            IrCaseBranch {
                pattern: IrPattern::Nil,
                guard_ops: None,
                ops: then_ops,
            },
            IrCaseBranch {
                pattern: IrPattern::Wildcard,
                guard_ops: None,
                ops: else_ops,
            },
        ],
        offset,
    });

    Ok(())
}

fn lower_pipe(
    left: &Expr,
    right: &Expr,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    match right {
        Expr::Call { callee, args, .. } => {
            // Prepend the left-hand side as the first argument
            let mut new_args = vec![left.clone()];
            new_args.extend_from_slice(args);
            lower_call(callee, &new_args, current_module, struct_definitions, ops, offset)
        }
        Expr::Variable { name, .. } => {
            // Pipe into a bare function name (zero-arg call with left as first arg)
            lower_expr(left, current_module, struct_definitions, ops)?;
            let call_target = if name.contains('.') {
                IrCallTarget::Function { name: name.clone() }
            } else {
                IrCallTarget::Function {
                    name: qualify_function_name(current_module, name),
                }
            };
            ops.push(IrOp::Call {
                callee: call_target,
                argc: 1,
                offset,
            });
            Ok(())
        }
        other => Err(LoweringError::unsupported(
            "pipe right-hand side",
            other.offset(),
        )),
    }
}

fn lower_binary(
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    match op {
        BinaryOp::And => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::And { right_ops, offset });
        }
        BinaryOp::Or => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::Or { right_ops, offset });
        }
        BinaryOp::AndAnd => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::AndAnd { right_ops, offset });
        }
        BinaryOp::OrOr => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            let mut right_ops = Vec::new();
            lower_expr(right, current_module, struct_definitions, &mut right_ops)?;
            ops.push(IrOp::OrOr { right_ops, offset });
        }
        BinaryOp::Add => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::AddInt { offset });
        }
        BinaryOp::Sub => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::SubInt { offset });
        }
        BinaryOp::Mul => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::MulInt { offset });
        }
        BinaryOp::Div => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::DivInt { offset });
        }
        BinaryOp::Eq => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::Eq,
                offset,
            });
        }
        BinaryOp::NotEq => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::NotEq,
                offset,
            });
        }
        BinaryOp::Lt => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::Lt,
                offset,
            });
        }
        BinaryOp::Lte => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::Lte,
                offset,
            });
        }
        BinaryOp::Gt => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::Gt,
                offset,
            });
        }
        BinaryOp::Gte => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::Gte,
                offset,
            });
        }
        BinaryOp::StrictEq => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::StrictEq,
                offset,
            });
        }
        BinaryOp::StrictNotEq => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::CmpInt {
                kind: CmpKind::StrictNotEq,
                offset,
            });
        }
        BinaryOp::Concat => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Concat { offset });
        }
        BinaryOp::In => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::In { offset });
        }
        BinaryOp::NotIn => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::NotIn { offset });
        }
        BinaryOp::PlusPlus => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::PlusPlus { offset });
        }
        BinaryOp::MinusMinus => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::MinusMinus { offset });
        }
        BinaryOp::Range => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::Range { offset });
        }
        BinaryOp::BitwiseAnd => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::BitwiseAnd { offset });
        }
        BinaryOp::BitwiseOr => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::BitwiseOr { offset });
        }
        BinaryOp::BitwiseXor => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::BitwiseXor { offset });
        }
        BinaryOp::BitwiseShiftLeft => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::BitwiseShiftLeft { offset });
        }
        BinaryOp::BitwiseShiftRight => {
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::BitwiseShiftRight { offset });
        }
        BinaryOp::SteppedRange => {
            // left is a Range, right is the step
            lower_expr(left, current_module, struct_definitions, ops)?;
            lower_expr(right, current_module, struct_definitions, ops)?;
            ops.push(IrOp::SteppedRange { offset });
        }
    }

    Ok(())
}

fn lower_unary(
    op: &crate::parser::UnaryOp,
    value: &Expr,
    current_module: &str,
    struct_definitions: &StructDefinitions,
    ops: &mut Vec<IrOp>,
    offset: usize,
) -> Result<(), LoweringError> {
    lower_expr(value, current_module, struct_definitions, ops)?;
    match op {
        crate::parser::UnaryOp::Not => ops.push(IrOp::Not { offset }),
        crate::parser::UnaryOp::Bang => ops.push(IrOp::Bang { offset }),
        crate::parser::UnaryOp::Neg => {
            // Unary negation: push 0 then subtract
            ops.push(IrOp::ConstInt { value: 0, offset });
            // Swap: we need 0 - value, but we pushed value first
            // Use a trick: push 0, then sub would do (top-1) - top = 0 - value if reversed
            // Actually SubInt pops right then left: right=top, left=second
            // We pushed: value, 0; pop right=0, left=value -> value - 0, wrong
            // Need: push 0 FIRST, then value, then SubInt gives 0 - value
            // So we need to re-order. Let's just multiply by -1 instead
            ops.pop(); // remove the ConstInt 0
            ops.push(IrOp::ConstInt { value: -1, offset });
            ops.push(IrOp::MulInt { offset });
        }
        crate::parser::UnaryOp::BitwiseNot => ops.push(IrOp::BitwiseNot { offset }),
    }
    Ok(())
}

fn lower_pattern(pattern: &Pattern) -> Result<IrPattern, LoweringError> {
    match pattern {
        Pattern::Wildcard { .. } => Ok(IrPattern::Wildcard),
        Pattern::Bind { name, .. } => Ok(IrPattern::Bind { name: name.clone() }),
        Pattern::Pin { name, .. } => Ok(IrPattern::Pin { name: name.clone() }),
        Pattern::Atom { value, .. } => Ok(IrPattern::Atom { value: value.clone() }),
        Pattern::Int { value, .. } => Ok(IrPattern::Integer { value: *value }),
        Pattern::Bool { value, .. } => Ok(IrPattern::Bool { value: *value }),
        Pattern::Nil { .. } => Ok(IrPattern::Nil),
        Pattern::String { value, .. } => Ok(IrPattern::String { value: value.clone() }),
        Pattern::Tuple { items, offset } => {
            if items.len() != 2 {
                return Err(LoweringError::unsupported("tuple pattern arity", *offset));
            }
            let lowered: Result<Vec<_>, _> = items.iter().map(lower_pattern).collect();
            Ok(IrPattern::Tuple { items: lowered? })
        }
        Pattern::List { items, tail, .. } => {
            let lowered_items: Result<Vec<_>, _> = items.iter().map(lower_pattern).collect();
            let lowered_tail = match tail {
                Some(tail) => Some(Box::new(lower_pattern(tail)?)),
                None => None,
            };
            Ok(IrPattern::List {
                items: lowered_items?,
                tail: lowered_tail,
            })
        }
        Pattern::Map { entries, .. } => {
            let lowered: Result<Vec<_>, _> = entries
                .iter()
                .map(|entry| {
                    Ok(IrMapPatternEntry {
                        key: lower_pattern(&entry.key)?,
                        value: lower_pattern(&entry.value)?,
                    })
                })
                .collect();
            Ok(IrPattern::Map { entries: lowered? })
        }
        Pattern::Struct { module, entries, .. } => {
            // Lower a struct pattern as a map pattern with a __struct__ key
            let mut map_entries = vec![IrMapPatternEntry {
                key: IrPattern::Atom {
                    value: "__struct__".to_string(),
                },
                value: IrPattern::Atom {
                    value: module.clone(),
                },
            }];

            for entry in entries {
                map_entries.push(IrMapPatternEntry {
                    key: IrPattern::Atom {
                        value: entry.key.clone(),
                    },
                    value: lower_pattern(&entry.value)?,
                });
            }

            Ok(IrPattern::Map {
                entries: map_entries,
            })
        }
        Pattern::Bitstring { segments, .. } => {
            let lowered: Result<Vec<_>, _> = segments
                .iter()
                .map(|seg| match seg {
                    crate::parser::BitstringPatternSegment::Literal { value, .. } => {
                        // Literal bytes: the value is an int literal
                        if *value < 0 || *value > 255 {
                            Err(LoweringError::unsupported(
                                "bitstring literal out of byte range",
                                0,
                            ))
                        } else {
                            Ok(IrBitstringSegment::Literal {
                                value: *value as u8,
                            })
                        }
                    }
                    crate::parser::BitstringPatternSegment::Bind { name, .. } => {
                        Ok(IrBitstringSegment::Bind { name: name.clone() })
                    }
                    crate::parser::BitstringPatternSegment::Wildcard { .. } => {
                        Ok(IrBitstringSegment::Wildcard)
                    }
                })
                .collect();
            Ok(IrPattern::Bitstring { segments: lowered? })
        }
    }
}

fn qualify_function_name(module: &str, function: &str) -> String {
    format!("{module}.{function}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_tokens;

    fn compile(source: &str) -> IrProgram {
        let tokens = scan_tokens(source).expect("lex error");
        let ast = parse_tokens(&tokens).expect("parse error");
        lower_ast_to_ir(&ast).expect("lowering error")
    }

    #[test]
    fn test_lower_simple_function() {
        let program = compile(
            r#"
            defmodule Demo do
              def run do
                42
              end
            end
            "#,
        );

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "Demo.run");
    }

    #[test]
    fn test_lower_function_with_args() {
        let program = compile(
            r#"
            defmodule Demo do
              def add(a, b) do
                a + b
              end
            end
            "#,
        );

        assert_eq!(program.functions.len(), 1);
        let func = &program.functions[0];
        assert_eq!(func.name, "Demo.add");
        assert_eq!(func.params, vec!["a", "b"]);
    }

    #[test]
    fn test_lower_case_expression() {
        let program = compile(
            r#"
            defmodule Demo do
              def run do
                case :ok do
                  :ok -> 1
                  :err -> 2
                end
              end
            end
            "#,
        );

        let ops = &program.functions[0].ops;
        let has_case = ops.iter().any(|op| matches!(op, IrOp::Case { .. }));
        assert!(has_case, "expected a Case op");
    }

    #[test]
    fn test_protocol_dispatch_function_has_protocol_dispatch_builtin() {
        let source = r#"
            defmodule MyProtocol do
              defprotocol do
                def my_func(x)
              end

              defimpl MyProtocol, for: Map do
                def my_func(x) do
                  x
                end
              end
            end
        "#;

        let tokens = scan_tokens(source).expect("lex");
        let ast = parse_tokens(&tokens).expect("parse");
        let program = lower_ast_to_ir(&ast).expect("lower");

        let dispatch_fn = program
            .functions
            .iter()
            .find(|f| f.name == "MyProtocol.my_func")
            .expect("dispatch function should exist");

        let serialized_ops = serde_json::to_string(&dispatch_fn.ops)
            .expect("ops should serialize");
        assert!(serialized_ops.contains("case"));

        // Also check that a protocol dispatch size function is generated properly
        let size_function = program
            .functions
            .iter()
            .find(|f| f.name == "MyProtocol.my_func")
            .expect("should find protocol dispatch function");

        let serialized_ops = serde_json::to_string(&size_function["ops"])
            .expect("protocol dispatcher ops should serialize");
        assert!(serialized_ops.contains("protocol_dispatch"));
    }
}
