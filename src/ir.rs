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
    IntDiv {
        offset: usize,
    },
    RemInt {
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
    /// Discard the top stack value (used between sequential expressions in a block).
    Drop,
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

#[path = "ir_collect.rs"]
mod collect;
use collect::*;

#[path = "ir_lower.rs"]
mod lower;
use lower::*;

#[path = "ir_lower_expr.rs"]
mod lower_expr;
use lower_expr::lower_expr;

#[path = "ir_patterns.rs"]
mod patterns;
use patterns::*;

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;
