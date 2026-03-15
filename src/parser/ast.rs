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
    Block {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        exprs: Vec<Expr>,
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
    Bitstring {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        items: Vec<Expr>,
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
    pub(crate) fn new(pattern: Pattern, source: Expr, guard: Option<Expr>) -> Self {
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

    pub(crate) fn source_mut(&mut self) -> &mut Expr {
        &mut self.source
    }

    pub(crate) fn guard_mut(&mut self) -> Option<&mut Expr> {
        self.guard.as_mut()
    }
}

pub type CaseBranch = Branch<Pattern>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch<Head>
where
    Head: BranchHead,
{
    pub(crate) head: Head,
    pub(crate) guard: Option<Expr>,
    pub(crate) body: Expr,
}

impl<Head> Branch<Head>
where
    Head: BranchHead,
{
    pub(crate) fn new(head: Head, guard: Option<Expr>, body: Expr) -> Self {
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

    pub(crate) fn guard_mut(&mut self) -> Option<&mut Expr> {
        self.guard.as_mut()
    }

    pub(crate) fn body_mut(&mut self) -> &mut Expr {
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

impl Expr {
    pub(crate) fn int(id: NodeId, offset: usize, value: i64) -> Self {
        Self::Int { id, offset, value }
    }

    pub(crate) fn float(id: NodeId, offset: usize, value: String) -> Self {
        Self::Float { id, offset, value }
    }

    pub(crate) fn bool(id: NodeId, offset: usize, value: bool) -> Self {
        Self::Bool { id, offset, value }
    }

    pub(crate) fn nil(id: NodeId, offset: usize) -> Self {
        Self::Nil { id, offset }
    }

    pub(crate) fn string(id: NodeId, offset: usize, value: String) -> Self {
        Self::String { id, offset, value }
    }

    pub(crate) fn interpolated_string(
        id: NodeId,
        offset: usize,
        segments: Vec<InterpolationSegment>,
    ) -> Self {
        Self::InterpolatedString {
            id,
            offset,
            segments,
        }
    }

    pub(crate) fn tuple(id: NodeId, offset: usize, items: Vec<Expr>) -> Self {
        Self::Tuple { id, offset, items }
    }

    pub(crate) fn list(id: NodeId, offset: usize, items: Vec<Expr>) -> Self {
        Self::List { id, offset, items }
    }

    pub(crate) fn bitstring(id: NodeId, offset: usize, items: Vec<Expr>) -> Self {
        Self::Bitstring { id, offset, items }
    }

    pub(crate) fn map(id: NodeId, offset: usize, entries: Vec<MapExprEntry>) -> Self {
        Self::Map {
            id,
            offset,
            entries,
        }
    }

    pub(crate) fn struct_literal(
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

    pub(crate) fn map_update(
        id: NodeId,
        offset: usize,
        base: Expr,
        updates: Vec<LabelExprEntry>,
    ) -> Self {
        Self::MapUpdate {
            id,
            offset,
            base: Box::new(base),
            updates,
        }
    }

    pub(crate) fn struct_update(
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

    pub(crate) fn keyword(id: NodeId, offset: usize, entries: Vec<LabelExprEntry>) -> Self {
        Self::Keyword {
            id,
            offset,
            entries,
        }
    }

    pub(crate) fn call(id: NodeId, offset: usize, callee: String, args: Vec<Expr>) -> Self {
        Self::Call {
            id,
            offset,
            callee,
            args,
        }
    }

    pub(crate) fn field_access(id: NodeId, offset: usize, base: Expr, label: String) -> Self {
        Self::FieldAccess {
            id,
            offset,
            base: Box::new(base),
            label,
        }
    }

    pub(crate) fn index_access(id: NodeId, offset: usize, base: Expr, index: Expr) -> Self {
        Self::IndexAccess {
            id,
            offset,
            base: Box::new(base),
            index: Box::new(index),
        }
    }

    pub(crate) fn anonymous_fn(id: NodeId, offset: usize, params: Vec<String>, body: Expr) -> Self {
        Self::Fn {
            id,
            offset,
            params,
            body: Box::new(body),
        }
    }

    pub(crate) fn invoke(id: NodeId, offset: usize, callee: Expr, args: Vec<Expr>) -> Self {
        Self::Invoke {
            id,
            offset,
            callee: Box::new(callee),
            args,
        }
    }

    pub(crate) fn question(id: NodeId, offset: usize, value: Expr) -> Self {
        Self::Question {
            id,
            offset,
            value: Box::new(value),
        }
    }

    pub(crate) fn group(id: NodeId, offset: usize, inner: Expr) -> Self {
        Self::Group {
            id,
            offset,
            inner: Box::new(inner),
        }
    }

    pub(crate) fn unary(id: NodeId, offset: usize, op: UnaryOp, value: Expr) -> Self {
        Self::Unary {
            id,
            offset,
            op,
            value: Box::new(value),
        }
    }

    pub(crate) fn binary(id: NodeId, op: BinaryOp, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Binary {
            id,
            offset,
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub(crate) fn pipe(id: NodeId, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Pipe {
            id,
            offset,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub(crate) fn case(
        id: NodeId,
        offset: usize,
        subject: Expr,
        branches: Vec<CaseBranch>,
    ) -> Self {
        Self::Case {
            id,
            offset,
            subject: Box::new(subject),
            branches,
        }
    }

    pub(crate) fn try_expr(
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

    pub(crate) fn raise(id: NodeId, offset: usize, error: Expr) -> Self {
        Self::Raise {
            id,
            offset,
            error: Box::new(error),
        }
    }

    pub(crate) fn for_comprehension(
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

    pub(crate) fn variable(id: NodeId, offset: usize, name: String) -> Self {
        Self::Variable { id, offset, name }
    }

    pub(crate) fn atom(id: NodeId, offset: usize, value: String) -> Self {
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
            | Self::For { offset, .. }
            | Self::Block { offset, .. }
            | Self::Bitstring { offset, .. } => *offset,
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
