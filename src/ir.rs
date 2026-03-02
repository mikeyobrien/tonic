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