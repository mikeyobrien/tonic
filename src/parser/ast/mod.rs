use crate::lexer::{Span, Token};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Ast {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) String);

impl NodeId {
    fn new(kind: &'static str, value: u64) -> Self {
        Self(format!("{kind}-{value:04}"))
    }
}

#[derive(Debug, Default)]
pub(crate) struct NodeIdGenerator {
    next: u64,
}

impl NodeIdGenerator {
    pub(crate) fn next_module(&mut self) -> NodeId {
        self.next("module")
    }

    pub(crate) fn next_function(&mut self) -> NodeId {
        self.next("function")
    }

    pub(crate) fn next_expr(&mut self) -> NodeId {
        self.next("expr")
    }

    fn next(&mut self, kind: &'static str) -> NodeId {
        self.next += 1;
        NodeId::new(kind, self.next)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ModuleForm {
    Alias {
        module: String,
        #[serde(rename = "as")]
        as_name: String,
    },
    Import {
        module: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        only: Option<Vec<ImportFunctionSpec>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        except: Option<Vec<ImportFunctionSpec>>,
    },
    Require {
        module: String,
    },
    Use {
        module: String,
    },
    Defstruct {
        fields: Vec<StructFieldEntry>,
    },
    Defprotocol {
        name: String,
        functions: Vec<ProtocolFunctionSignature>,
    },
    Defimpl {
        protocol: String,
        #[serde(rename = "for")]
        target: String,
        functions: Vec<ProtocolImplFunction>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct ImportFunctionSpec {
    pub name: String,
    pub arity: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructFieldEntry {
    pub name: String,
    pub default: Expr,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProtocolFunctionSignature {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProtocolImplFunction {
    pub name: String,
    pub params: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModuleAttribute {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Module {
    #[serde(skip_serializing)]
    pub id: NodeId,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<ModuleForm>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<ModuleAttribute>,
    pub functions: Vec<Function>,
}

impl Module {
    pub(crate) fn with_id(
        id: NodeId,
        name: String,
        forms: Vec<ModuleForm>,
        attributes: Vec<ModuleAttribute>,
        functions: Vec<Function>,
    ) -> Self {
        Self {
            id,
            name,
            forms,
            attributes,
            functions,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FunctionVisibility {
    Public,
    Private,
}

impl FunctionVisibility {
    fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Function {
    #[serde(skip_serializing)]
    pub id: NodeId,
    pub name: String,
    #[serde(skip_serializing_if = "FunctionVisibility::is_public")]
    pub visibility: FunctionVisibility,
    pub params: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Expr>,
    pub body: Expr,
}

impl Function {
    pub(crate) fn with_id(
        id: NodeId,
        name: String,
        visibility: FunctionVisibility,
        params: Vec<Parameter>,
        guard: Option<Expr>,
        body: Expr,
    ) -> Self {
        Self {
            id,
            name,
            visibility,
            params,
            guard,
            body,
        }
    }

    pub fn guard(&self) -> Option<&Expr> {
        self.guard.as_ref()
    }

    pub fn is_private(&self) -> bool {
        matches!(self.visibility, FunctionVisibility::Private)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterAnnotation {
    Inferred,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    name: String,
    annotation: ParameterAnnotation,
    pattern: Pattern,
    default: Option<Expr>,
}

impl Parameter {
    pub(crate) fn inferred(name: String, pattern: Pattern, default: Option<Expr>) -> Self {
        Self {
            name,
            annotation: ParameterAnnotation::Inferred,
            pattern,
            default,
        }
    }

    pub(crate) fn dynamic(name: String, default: Option<Expr>) -> Self {
        Self {
            pattern: Pattern::Bind { name: name.clone() },
            name,
            annotation: ParameterAnnotation::Dynamic,
            default,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn annotation(&self) -> ParameterAnnotation {
        self.annotation
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn default(&self) -> Option<&Expr> {
        self.default.as_ref()
    }

    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }

    pub(crate) fn default_mut(&mut self) -> Option<&mut Expr> {
        self.default.as_mut()
    }
}

impl Serialize for Parameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.default.is_none()
            && matches!(&self.pattern, Pattern::Bind { name } if name == &self.name)
        {
            return serializer.serialize_str(&self.name);
        }

        use serde::ser::SerializeStruct;

        let mut parameter = serializer.serialize_struct("Parameter", 4)?;
        parameter.serialize_field("name", &self.name)?;
        parameter.serialize_field("pattern", &self.pattern)?;
        if matches!(self.annotation, ParameterAnnotation::Dynamic) {
            parameter.serialize_field("annotation", "dynamic")?;
        }
        if let Some(default) = &self.default {
            parameter.serialize_field("default", default)?;
        }
        parameter.end()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum InterpolationSegment {
    String { value: String },
    Expr { expr: Expr },
}

mod expr_def;
pub use expr_def::*;
mod expr_impl;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Pattern {
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
        items: Vec<Pattern>,
    },
    List {
        items: Vec<Pattern>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tail: Option<Box<Pattern>>,
    },
    Bitstring {
        items: Vec<Pattern>,
    },
    Map {
        entries: Vec<MapPatternEntry>,
    },
    Struct {
        module: String,
        entries: Vec<LabelPatternEntry>,
    },
}

impl BranchHead for Pattern {
    const FIELD_NAME: &'static str = "pattern";
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MapPatternEntry {
    key: Pattern,
    value: Pattern,
}

impl MapPatternEntry {
    pub(crate) fn new(key: Pattern, value: Pattern) -> Self {
        Self { key, value }
    }

    pub(crate) fn key(&self) -> &Pattern {
        &self.key
    }

    pub(crate) fn value(&self) -> &Pattern {
        &self.value
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LabelPatternEntry {
    pub(crate) key: String,
    pub(crate) value: Pattern,
}

impl LabelPatternEntry {
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn value(&self) -> &Pattern {
        &self.value
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MapExprEntry {
    pub(crate) key: Expr,
    pub(crate) value: Expr,
}

impl MapExprEntry {
    pub(crate) fn key(&self) -> &Expr {
        &self.key
    }

    pub(crate) fn value(&self) -> &Expr {
        &self.value
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LabelExprEntry {
    pub(crate) key: String,
    pub(crate) value: Expr,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UnaryOp {
    Not,
    Bang,
    Plus,
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BinaryOp {
    Match,
    Plus,
    Minus,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    AndAnd,
    OrOr,
    Concat,
    PlusPlus,
    MinusMinus,
    In,
    NotIn,
    Range,
    StrictEq,
    StrictBangEq,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseShiftLeft,
    BitwiseShiftRight,
    SteppedRange,
    IntDiv,
    Rem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    message: String,
    span: Option<Span>,
}

impl ParserError {
    pub(crate) fn at_current(message: impl Into<String>, token: Option<&Token>) -> Self {
        Self {
            message: message.into(),
            span: token.map(Token::span),
        }
    }

    pub fn offset(&self) -> Option<usize> {
        self.span.map(|span| span.start())
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "{} at offset {}", self.message, span.start())
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ParserError {}
