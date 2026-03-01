use crate::guard_builtins;
use crate::lexer::{Span, Token, TokenKind};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;

const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";
const RESCUE_EXCEPTION_BINDING: &str = "__tonic_rescue_exception";

fn starts_with_uppercase(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Ast {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    fn new(kind: &'static str, value: u64) -> Self {
        Self(format!("{kind}-{value:04}"))
    }
}

#[derive(Debug, Default)]
struct NodeIdGenerator {
    next: u64,
}

impl NodeIdGenerator {
    fn next_module(&mut self) -> NodeId {
        self.next("module")
    }

    fn next_function(&mut self) -> NodeId {
        self.next("function")
    }

    fn next_expr(&mut self) -> NodeId {
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
    fn with_id(
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
    fn with_id(
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
    fn inferred(name: String, pattern: Pattern, default: Option<Expr>) -> Self {
        Self {
            name,
            annotation: ParameterAnnotation::Inferred,
            pattern,
            default,
        }
    }

    fn dynamic(name: String, default: Option<Expr>) -> Self {
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Expr {
    Int {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: i64,
    },
    Float {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: String,
    },
    Bool {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: bool,
    },
    Nil {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
    },
    String {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: String,
    },
    InterpolatedString {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        segments: Vec<InterpolationSegment>,
    },
    Tuple {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        items: Vec<Expr>,
    },
    List {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        items: Vec<Expr>,
    },
    Map {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        entries: Vec<MapExprEntry>,
    },
    Struct {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        module: String,
        entries: Vec<LabelExprEntry>,
    },
    MapUpdate {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        base: Box<Expr>,
        updates: Vec<LabelExprEntry>,
    },
    StructUpdate {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        module: String,
        base: Box<Expr>,
        updates: Vec<LabelExprEntry>,
    },
    Keyword {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        entries: Vec<LabelExprEntry>,
    },
    Call {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        callee: String,
        args: Vec<Expr>,
    },
    FieldAccess {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        base: Box<Expr>,
        label: String,
    },
    IndexAccess {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Fn {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        params: Vec<String>,
        body: Box<Expr>,
    },
    Invoke {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Question {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: Box<Expr>,
    },
    Group {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        inner: Box<Expr>,
    },
    Binary {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        op: UnaryOp,
        value: Box<Expr>,
    },
    Pipe {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Case {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        subject: Box<Expr>,
        branches: Vec<CaseBranch>,
    },
    Try {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        body: Box<Expr>,
        rescue: Vec<CaseBranch>,
        catch: Vec<CaseBranch>,
        after: Option<Box<Expr>>,
    },
    Raise {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        error: Box<Expr>,
    },
    For {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        generators: Vec<ForGenerator>,
        into: Option<Box<Expr>>,
        reduce: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Variable {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        name: String,
    },
    Atom {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ForGenerator {
    pattern: Pattern,
    source: Expr,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard: Option<Expr>,
}

impl ForGenerator {
    fn new(pattern: Pattern, source: Expr, guard: Option<Expr>) -> Self {
        Self {
            pattern,
            source,
            guard,
        }
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn source(&self) -> &Expr {
        &self.source
    }

    pub fn guard(&self) -> Option<&Expr> {
        self.guard.as_ref()
    }

    fn source_mut(&mut self) -> &mut Expr {
        &mut self.source
    }

    fn guard_mut(&mut self) -> Option<&mut Expr> {
        self.guard.as_mut()
    }
}

pub type CaseBranch = Branch<Pattern>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch<Head>
where
    Head: BranchHead,
{
    head: Head,
    guard: Option<Expr>,
    body: Expr,
}

impl<Head> Branch<Head>
where
    Head: BranchHead,
{
    fn new(head: Head, guard: Option<Expr>, body: Expr) -> Self {
        Self { head, guard, body }
    }

    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn guard(&self) -> Option<&Expr> {
        self.guard.as_ref()
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }

    fn guard_mut(&mut self) -> Option<&mut Expr> {
        self.guard.as_mut()
    }

    fn body_mut(&mut self) -> &mut Expr {
        &mut self.body
    }
}

impl<Head> Serialize for Branch<Head>
where
    Head: BranchHead,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut branch = serializer.serialize_struct("Branch", 3)?;
        branch.serialize_field(Head::FIELD_NAME, self.head())?;
        if let Some(guard) = self.guard() {
            branch.serialize_field("guard", guard)?;
        }
        branch.serialize_field("body", self.body())?;
        branch.end()
    }
}

pub trait BranchHead: Serialize {
    const FIELD_NAME: &'static str;
}

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

impl Expr {
    fn int(id: NodeId, offset: usize, value: i64) -> Self {
        Self::Int { id, offset, value }
    }

    fn float(id: NodeId, offset: usize, value: String) -> Self {
        Self::Float { id, offset, value }
    }

    fn bool(id: NodeId, offset: usize, value: bool) -> Self {
        Self::Bool { id, offset, value }
    }

    fn nil(id: NodeId, offset: usize) -> Self {
        Self::Nil { id, offset }
    }

    fn string(id: NodeId, offset: usize, value: String) -> Self {
        Self::String { id, offset, value }
    }

    fn interpolated_string(id: NodeId, offset: usize, segments: Vec<InterpolationSegment>) -> Self {
        Self::InterpolatedString {
            id,
            offset,
            segments,
        }
    }

    fn tuple(id: NodeId, offset: usize, items: Vec<Expr>) -> Self {
        Self::Tuple { id, offset, items }
    }

    fn list(id: NodeId, offset: usize, items: Vec<Expr>) -> Self {
        Self::List { id, offset, items }
    }

    fn map(id: NodeId, offset: usize, entries: Vec<MapExprEntry>) -> Self {
        Self::Map {
            id,
            offset,
            entries,
        }
    }

    fn struct_literal(
        id: NodeId,
        offset: usize,
        module: String,
        entries: Vec<LabelExprEntry>,
    ) -> Self {
        Self::Struct {
            id,
            offset,
            module,
            entries,
        }
    }

    fn map_update(id: NodeId, offset: usize, base: Expr, updates: Vec<LabelExprEntry>) -> Self {
        Self::MapUpdate {
            id,
            offset,
            base: Box::new(base),
            updates,
        }
    }

    fn struct_update(
        id: NodeId,
        offset: usize,
        module: String,
        base: Expr,
        updates: Vec<LabelExprEntry>,
    ) -> Self {
        Self::StructUpdate {
            id,
            offset,
            module,
            base: Box::new(base),
            updates,
        }
    }

    fn keyword(id: NodeId, offset: usize, entries: Vec<LabelExprEntry>) -> Self {
        Self::Keyword {
            id,
            offset,
            entries,
        }
    }

    fn call(id: NodeId, offset: usize, callee: String, args: Vec<Expr>) -> Self {
        Self::Call {
            id,
            offset,
            callee,
            args,
        }
    }

    fn field_access(id: NodeId, offset: usize, base: Expr, label: String) -> Self {
        Self::FieldAccess {
            id,
            offset,
            base: Box::new(base),
            label,
        }
    }

    fn index_access(id: NodeId, offset: usize, base: Expr, index: Expr) -> Self {
        Self::IndexAccess {
            id,
            offset,
            base: Box::new(base),
            index: Box::new(index),
        }
    }

    fn anonymous_fn(id: NodeId, offset: usize, params: Vec<String>, body: Expr) -> Self {
        Self::Fn {
            id,
            offset,
            params,
            body: Box::new(body),
        }
    }

    fn invoke(id: NodeId, offset: usize, callee: Expr, args: Vec<Expr>) -> Self {
        Self::Invoke {
            id,
            offset,
            callee: Box::new(callee),
            args,
        }
    }

    fn question(id: NodeId, offset: usize, value: Expr) -> Self {
        Self::Question {
            id,
            offset,
            value: Box::new(value),
        }
    }

    fn group(id: NodeId, offset: usize, inner: Expr) -> Self {
        Self::Group {
            id,
            offset,
            inner: Box::new(inner),
        }
    }

    fn unary(id: NodeId, offset: usize, op: UnaryOp, value: Expr) -> Self {
        Self::Unary {
            id,
            offset,
            op,
            value: Box::new(value),
        }
    }

    fn binary(id: NodeId, op: BinaryOp, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Binary {
            id,
            offset,
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn pipe(id: NodeId, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Pipe {
            id,
            offset,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn case(id: NodeId, offset: usize, subject: Expr, branches: Vec<CaseBranch>) -> Self {
        Self::Case {
            id,
            offset,
            subject: Box::new(subject),
            branches,
        }
    }

    fn try_expr(
        id: NodeId,
        offset: usize,
        body: Expr,
        rescue: Vec<CaseBranch>,
        catch: Vec<CaseBranch>,
        after: Option<Expr>,
    ) -> Self {
        Self::Try {
            id,
            offset,
            body: Box::new(body),
            rescue,
            catch,
            after: after.map(Box::new),
        }
    }

    fn raise(id: NodeId, offset: usize, error: Expr) -> Self {
        Self::Raise {
            id,
            offset,
            error: Box::new(error),
        }
    }

    fn for_comprehension(
        id: NodeId,
        offset: usize,
        generators: Vec<ForGenerator>,
        into: Option<Expr>,
        reduce: Option<Expr>,
        body: Expr,
    ) -> Self {
        Self::For {
            id,
            offset,
            generators,
            into: into.map(Box::new),
            reduce: reduce.map(Box::new),
            body: Box::new(body),
        }
    }

    fn variable(id: NodeId, offset: usize, name: String) -> Self {
        Self::Variable { id, offset, name }
    }

    fn atom(id: NodeId, offset: usize, value: String) -> Self {
        Self::Atom { id, offset, value }
    }

    pub fn offset(&self) -> usize {
        match self {
            Self::Int { offset, .. }
            | Self::Float { offset, .. }
            | Self::Bool { offset, .. }
            | Self::Nil { offset, .. }
            | Self::String { offset, .. }
            | Self::InterpolatedString { offset, .. }
            | Self::Tuple { offset, .. }
            | Self::List { offset, .. }
            | Self::Map { offset, .. }
            | Self::Struct { offset, .. }
            | Self::MapUpdate { offset, .. }
            | Self::StructUpdate { offset, .. }
            | Self::Keyword { offset, .. }
            | Self::Call { offset, .. }
            | Self::FieldAccess { offset, .. }
            | Self::IndexAccess { offset, .. }
            | Self::Fn { offset, .. }
            | Self::Invoke { offset, .. }
            | Self::Question { offset, .. }
            | Self::Group { offset, .. }
            | Self::Binary { offset, .. }
            | Self::Unary { offset, .. }
            | Self::Pipe { offset, .. }
            | Self::Variable { offset, .. }
            | Self::Atom { offset, .. }
            | Self::Case { offset, .. }
            | Self::Try { offset, .. }
            | Self::Raise { offset, .. }
            | Self::For { offset, .. } => *offset,
        }
    }
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    message: String,
    span: Option<Span>,
}

impl ParserError {
    fn at_current(message: impl Into<String>, token: Option<&Token>) -> Self {
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

fn collect_module_callable_signatures(
    modules: &[Module],
) -> HashMap<String, HashSet<(String, usize)>> {
    let mut callable = HashMap::new();

    for module in modules {
        let mut signatures = HashSet::new();

        for function in &module.functions {
            if function.is_private() {
                continue;
            }

            let max_arity = function.params.len();
            let default_count = function
                .params
                .iter()
                .rev()
                .take_while(|param| param.has_default())
                .count();
            let min_arity = max_arity.saturating_sub(default_count);

            for arity in min_arity..=max_arity {
                signatures.insert((function.name.clone(), arity));
            }
        }

        callable.insert(module.name.clone(), signatures);
    }

    callable
}

#[derive(Debug, Clone)]
struct ImportScope {
    module: String,
    only: Option<HashSet<(String, usize)>>,
    except: HashSet<(String, usize)>,
    exported_signatures: Option<HashSet<(String, usize)>>,
}

impl ImportScope {
    fn from_module_form(
        module: &str,
        only: &Option<Vec<ImportFunctionSpec>>,
        except: &Option<Vec<ImportFunctionSpec>>,
        callable_modules: &HashMap<String, HashSet<(String, usize)>>,
    ) -> Self {
        let only = only.as_ref().map(|entries| {
            entries
                .iter()
                .map(|entry| (entry.name.clone(), entry.arity))
                .collect::<HashSet<_>>()
        });

        let except = except
            .as_ref()
            .map(|entries| {
                entries
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.arity))
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        Self {
            module: module.to_string(),
            only,
            except,
            exported_signatures: callable_modules.get(module).cloned(),
        }
    }

    fn allows(&self, name: &str, arity: usize) -> bool {
        if self
            .exported_signatures
            .as_ref()
            .is_some_and(|signatures| !signatures.contains(&(name.to_string(), arity)))
        {
            return false;
        }
        if self.except.contains(&(name.to_string(), arity)) {
            return false;
        }

        if let Some(only) = &self.only {
            return only.contains(&(name.to_string(), arity));
        }

        true
    }
}

fn canonicalize_module_call_targets(
    module: &mut Module,
    callable_modules: &HashMap<String, HashSet<(String, usize)>>,
) {
    // Scoped module-form semantics (parity task 04):
    // - `import Module` keeps existing behavior for unqualified call rewriting.
    // - `use Module` provides a limited compile-time effect by acting as an import fallback
    //   only when the module has no explicit imports.
    // - Full Elixir `__using__/1` macro expansion is intentionally deferred.
    let aliases = module
        .forms
        .iter()
        .filter_map(|form| match form {
            ModuleForm::Alias { module, as_name } => Some((as_name.clone(), module.clone())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let mut imports = Vec::new();
    let mut use_fallback_modules = Vec::new();
    for form in &module.forms {
        match form {
            ModuleForm::Import {
                module,
                only,
                except,
            } => {
                imports.push(ImportScope::from_module_form(
                    module,
                    only,
                    except,
                    callable_modules,
                ));
            }
            ModuleForm::Use { module } => {
                if !use_fallback_modules.contains(module) {
                    use_fallback_modules.push(module.clone());
                }
            }
            _ => {}
        }
    }

    let local_functions = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();

    for form in &mut module.forms {
        match form {
            ModuleForm::Defstruct { fields } => {
                for field in fields {
                    canonicalize_expr_call_targets(
                        &mut field.default,
                        &aliases,
                        &imports,
                        &use_fallback_modules,
                        &local_functions,
                    );
                }
            }
            ModuleForm::Defimpl { functions, .. } => {
                for function in functions {
                    for param in &mut function.params {
                        if let Some(default) = &mut param.default {
                            canonicalize_expr_call_targets(
                                default,
                                &aliases,
                                &imports,
                                &use_fallback_modules,
                                &local_functions,
                            );
                        }
                    }

                    if let Some(guard) = &mut function.guard {
                        canonicalize_expr_call_targets(
                            guard,
                            &aliases,
                            &imports,
                            &use_fallback_modules,
                            &local_functions,
                        );
                    }

                    canonicalize_expr_call_targets(
                        &mut function.body,
                        &aliases,
                        &imports,
                        &use_fallback_modules,
                        &local_functions,
                    );
                }
            }
            _ => {}
        }
    }

    for function in &mut module.functions {
        for param in &mut function.params {
            if let Some(default) = &mut param.default {
                canonicalize_expr_call_targets(
                    default,
                    &aliases,
                    &imports,
                    &use_fallback_modules,
                    &local_functions,
                );
            }
        }

        if let Some(guard) = &mut function.guard {
            canonicalize_expr_call_targets(
                guard,
                &aliases,
                &imports,
                &use_fallback_modules,
                &local_functions,
            );
        }

        canonicalize_expr_call_targets(
            &mut function.body,
            &aliases,
            &imports,
            &use_fallback_modules,
            &local_functions,
        );
    }
}

fn canonicalize_expr_call_targets(
    expr: &mut Expr,
    aliases: &HashMap<String, String>,
    imports: &[ImportScope],
    use_fallback_modules: &[String],
    local_functions: &HashSet<String>,
) {
    match expr {
        Expr::Tuple { items, .. } | Expr::List { items, .. } => {
            for item in items {
                canonicalize_expr_call_targets(
                    item,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                canonicalize_expr_call_targets(
                    &mut entry.key,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
                canonicalize_expr_call_targets(
                    &mut entry.value,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::Struct { entries, .. } => {
            for entry in entries {
                canonicalize_expr_call_targets(
                    &mut entry.value,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::Keyword { entries, .. } => {
            for entry in entries {
                canonicalize_expr_call_targets(
                    &mut entry.value,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::MapUpdate { base, updates, .. } => {
            canonicalize_expr_call_targets(
                base,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            for entry in updates {
                canonicalize_expr_call_targets(
                    &mut entry.value,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::StructUpdate { base, updates, .. } => {
            canonicalize_expr_call_targets(
                base,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            for entry in updates {
                canonicalize_expr_call_targets(
                    &mut entry.value,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::FieldAccess { base, .. } => {
            canonicalize_expr_call_targets(
                base,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::IndexAccess { base, index, .. } => {
            canonicalize_expr_call_targets(
                base,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            canonicalize_expr_call_targets(
                index,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Call { callee, args, .. } => {
            let arity = args.len();
            for arg in args.iter_mut() {
                canonicalize_expr_call_targets(
                    arg,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }

            if let Some((alias_name, function_name)) = callee.split_once('.') {
                if let Some(module_name) = aliases.get(alias_name) {
                    *callee = format!("{module_name}.{function_name}");
                }
                return;
            }

            if local_functions.contains(callee) || is_builtin_call_target(callee) {
                return;
            }

            let mut import_matches = imports
                .iter()
                .filter(|scope| scope.allows(callee, arity))
                .map(|scope| scope.module.as_str())
                .collect::<Vec<_>>();
            import_matches.sort_unstable();
            import_matches.dedup();

            if import_matches.len() == 1 {
                *callee = format!("{}.{}", import_matches[0], callee);
            } else if imports.is_empty() && use_fallback_modules.len() == 1 {
                *callee = format!("{}.{}", use_fallback_modules[0], callee);
            }
        }
        Expr::Fn { body, .. } => {
            canonicalize_expr_call_targets(
                body,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Invoke { callee, args, .. } => {
            canonicalize_expr_call_targets(
                callee,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            for arg in args {
                canonicalize_expr_call_targets(
                    arg,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::Question { value, .. }
        | Expr::Group { inner: value, .. }
        | Expr::Unary { value, .. } => {
            canonicalize_expr_call_targets(
                value,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            canonicalize_expr_call_targets(
                left,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            canonicalize_expr_call_targets(
                right,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Case {
            subject, branches, ..
        } => {
            canonicalize_expr_call_targets(
                subject,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            for branch in branches {
                if let Some(guard) = branch.guard_mut() {
                    canonicalize_expr_call_targets(
                        guard,
                        aliases,
                        imports,
                        use_fallback_modules,
                        local_functions,
                    );
                }
                canonicalize_expr_call_targets(
                    branch.body_mut(),
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::For {
            generators,
            into,
            reduce,
            body,
            ..
        } => {
            for generator in generators {
                canonicalize_expr_call_targets(
                    generator.source_mut(),
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
                if let Some(guard) = generator.guard_mut() {
                    canonicalize_expr_call_targets(
                        guard,
                        aliases,
                        imports,
                        use_fallback_modules,
                        local_functions,
                    );
                }
            }
            if let Some(into_expr) = into {
                canonicalize_expr_call_targets(
                    into_expr,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
            if let Some(reduce_expr) = reduce {
                canonicalize_expr_call_targets(
                    reduce_expr,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
            canonicalize_expr_call_targets(
                body,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            canonicalize_expr_call_targets(
                body,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
            for branch in rescue {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr_call_targets(
                        guard,
                        aliases,
                        imports,
                        use_fallback_modules,
                        local_functions,
                    );
                }
                canonicalize_expr_call_targets(
                    &mut branch.body,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
            for branch in catch {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr_call_targets(
                        guard,
                        aliases,
                        imports,
                        use_fallback_modules,
                        local_functions,
                    );
                }
                canonicalize_expr_call_targets(
                    &mut branch.body,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
            if let Some(after) = after {
                canonicalize_expr_call_targets(
                    after,
                    aliases,
                    imports,
                    use_fallback_modules,
                    local_functions,
                );
            }
        }
        Expr::Raise { error, .. } => {
            canonicalize_expr_call_targets(
                error,
                aliases,
                imports,
                use_fallback_modules,
                local_functions,
            );
        }
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Nil { .. }
        | Expr::String { .. }
        | Expr::Variable { .. }
        | Expr::Atom { .. } => {}
        Expr::InterpolatedString { segments, .. } => {
            for segment in segments {
                if let InterpolationSegment::Expr { expr } = segment {
                    canonicalize_expr_call_targets(
                        expr,
                        aliases,
                        imports,
                        use_fallback_modules,
                        local_functions,
                    );
                }
            }
        }
    }
}

fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err" | "tuple" | "list" | "map" | "keyword" | "protocol_dispatch" | "host_call"
    ) || guard_builtins::is_guard_builtin(callee)
}

fn token_can_start_no_paren_arg(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident
            | TokenKind::Atom
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::StringStart
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::LParen
            | TokenKind::LBrace
            | TokenKind::LBracket
            | TokenKind::Percent
            | TokenKind::Fn
            | TokenKind::If
            | TokenKind::Unless
            | TokenKind::Case
            | TokenKind::Cond
            | TokenKind::With
            | TokenKind::For
            | TokenKind::Try
            | TokenKind::Raise
            | TokenKind::Ampersand
    )
}

pub fn parse_ast(tokens: &[Token]) -> Result<Ast, ParserError> {
    Parser::new(tokens).parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    node_ids: NodeIdGenerator,
    capture_param_max_stack: Vec<usize>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            index: 0,
            node_ids: NodeIdGenerator::default(),
            capture_param_max_stack: Vec::new(),
        }
    }

    fn parse_program(mut self) -> Result<Ast, ParserError> {
        let mut modules = Vec::new();

        while !self.is_at_end() {
            let mut parsed = self.parse_module_group(None)?;
            modules.append(&mut parsed);
        }

        let callable_modules = collect_module_callable_signatures(&modules);
        for module in &mut modules {
            canonicalize_module_call_targets(module, &callable_modules);
        }

        Ok(Ast { modules })
    }

    /// Parse a defmodule, returning a list of modules (parent + any nested ones).
    /// `parent_name` is Some("Outer") when parsing a nested module inside Outer.
    fn parse_module_group(&mut self, parent_name: Option<&str>) -> Result<Vec<Module>, ParserError> {
        let id = self.node_ids.next_module();

        self.expect(TokenKind::Defmodule, "defmodule")?;
        let local_name = self.expect_ident("module name")?;
        let name = if let Some(parent) = parent_name {
            format!("{parent}.{local_name}")
        } else {
            local_name
        };
        self.expect(TokenKind::Do, "do")?;

        let mut forms = Vec::new();
        let mut attributes = Vec::new();
        let mut functions = Vec::new();
        let mut extra_modules: Vec<Module> = Vec::new();

        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("module declaration"));
            }

            if self.check(TokenKind::Def) || self.check(TokenKind::Defp) {
                functions.push(self.parse_function()?);
                continue;
            }

            if self.current_starts_module_form() {
                forms.push(self.parse_module_form()?);
                continue;
            }

            if self.check(TokenKind::At) {
                attributes.push(self.parse_module_attribute()?);
                continue;
            }

            // Allow nested defmodule declarations
            if self.check(TokenKind::Defmodule) {
                let mut nested = self.parse_module_group(Some(&name))?;
                extra_modules.append(&mut nested);
                continue;
            }

            return Err(self.expected("module declaration"));
        }

        self.expect(TokenKind::End, "end")?;

        let parent_module = Module::with_id(id, name, forms, attributes, functions);
        let mut result = vec![parent_module];
        result.append(&mut extra_modules);
        Ok(result)
    }

    fn parse_function(&mut self) -> Result<Function, ParserError> {
        let id = self.node_ids.next_function();

        let visibility = if self.match_kind(TokenKind::Def) {
            FunctionVisibility::Public
        } else if self.match_kind(TokenKind::Defp) {
            FunctionVisibility::Private
        } else {
            return Err(self.expected("def or defp"));
        };

        let name = self.expect_ident("function name")?;
        self.expect(TokenKind::LParen, "(")?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen, ")")?;

        if self.check(TokenKind::Arrow)
            && self
                .peek(1)
                .map(|token| token.kind() == TokenKind::Ident && token.lexeme() == "dynamic")
                .unwrap_or(false)
        {
            return Err(ParserError::at_current(
                "dynamic annotation is only allowed on parameters",
                self.current(),
            ));
        }

        let guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_expression()?;
        self.expect(TokenKind::End, "end")?;

        Ok(Function::with_id(id, name, visibility, params, guard, body))
    }

    fn current_starts_module_form(&self) -> bool {
        self.current().is_some_and(|token| {
            token.kind() == TokenKind::Ident
                && matches!(
                    token.lexeme(),
                    "alias"
                        | "import"
                        | "require"
                        | "use"
                        | "defstruct"
                        | "defprotocol"
                        | "defimpl"
                )
        })
    }

    fn parse_module_form(&mut self) -> Result<ModuleForm, ParserError> {
        let form_name = self.expect_ident("module form")?;

        match form_name.as_str() {
            "alias" => self.parse_alias_form(),
            "import" => self.parse_import_form(),
            "require" => self.parse_named_module_form("require"),
            "use" => self.parse_named_module_form("use"),
            "defstruct" => self.parse_defstruct_form(),
            "defprotocol" => self.parse_defprotocol_form(),
            "defimpl" => self.parse_defimpl_form(),
            _ => Err(ParserError::at_current(
                format!("unsupported module form '{form_name}'"),
                self.current(),
            )),
        }
    }

    fn parse_alias_form(&mut self) -> Result<ModuleForm, ParserError> {
        let module = self.parse_module_reference("aliased module")?;
        let mut as_name = module.rsplit('.').next().unwrap_or(&module).to_string();

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "alias option")?;
            if option_token.lexeme() != "as" {
                return Err(ParserError::at_current(
                    format!(
                        "unsupported alias option '{}'; supported syntax: alias Module, as: Name",
                        option_token.lexeme()
                    ),
                    Some(option_token),
                ));
            }

            self.expect(TokenKind::Colon, ":")?;
            as_name = self.expect_ident("alias name")?;
        }

        Ok(ModuleForm::Alias { module, as_name })
    }

    fn parse_import_form(&mut self) -> Result<ModuleForm, ParserError> {
        let module = self.parse_module_reference("module name")?;
        let mut only = None;
        let mut except = None;

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "import option")?;
            let option_name = option_token.lexeme();
            if !matches!(option_name, "only" | "except") {
                return Err(ParserError::at_current(
                    format!(
                        "unsupported import option '{}'; supported syntax: import Module, only: [name: arity] or except: [name: arity]",
                        option_name
                    ),
                    Some(option_token),
                ));
            }

            self.expect(TokenKind::Colon, ":")?;
            let entries = self.parse_import_filter_entries(option_name)?;

            match option_name {
                "only" => only = Some(entries),
                "except" => except = Some(entries),
                _ => unreachable!("validated import option"),
            }

            if self.match_kind(TokenKind::Comma) {
                return Err(ParserError::at_current(
                    "import accepts exactly one filter option (`only:` or `except:`)",
                    self.current(),
                ));
            }
        }

        Ok(ModuleForm::Import {
            module,
            only,
            except,
        })
    }

    fn parse_import_filter_entries(
        &mut self,
        option_name: &str,
    ) -> Result<Vec<ImportFunctionSpec>, ParserError> {
        self.expect(TokenKind::LBracket, "[")?;

        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        if self.match_kind(TokenKind::RBracket) {
            return Ok(entries);
        }

        loop {
            let function_name = self
                .expect_token(TokenKind::Ident, "import filter function name")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?
                .lexeme()
                .to_string();
            self.expect(TokenKind::Colon, ":")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;
            let arity_token = self
                .expect_token(TokenKind::Integer, "import filter arity")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;
            let arity = arity_token
                .lexeme()
                .parse::<usize>()
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;

            if seen.insert((function_name.clone(), arity)) {
                entries.push(ImportFunctionSpec {
                    name: function_name,
                    arity,
                });
            }

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(TokenKind::RBracket, "]")
            .map_err(|_| self.invalid_import_filter_shape(option_name))?;