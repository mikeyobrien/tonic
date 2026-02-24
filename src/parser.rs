use crate::lexer::{Span, Token, TokenKind};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;

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
    },
    Require {
        module: String,
    },
    Use {
        module: String,
    },
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

#[derive(Debug, Serialize, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
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
        generators: Vec<(Pattern, Expr)>,
        into: Option<Box<Expr>>,
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

    fn map(id: NodeId, offset: usize, entries: Vec<LabelExprEntry>) -> Self {
        Self::Map {
            id,
            offset,
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
        generators: Vec<(Pattern, Expr)>,
        into: Option<Expr>,
        body: Expr,
    ) -> Self {
        Self::For {
            id,
            offset,
            generators,
            into: into.map(Box::new),
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
            | Self::MapUpdate { offset, .. }
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
    Range,
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

fn canonicalize_module_call_targets(module: &mut Module) {
    let aliases = module
        .forms
        .iter()
        .filter_map(|form| match form {
            ModuleForm::Alias { module, as_name } => Some((as_name.clone(), module.clone())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let imports = module
        .forms
        .iter()
        .filter_map(|form| match form {
            ModuleForm::Import { module } => Some(module.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let local_functions = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();

    for function in &mut module.functions {
        for param in &mut function.params {
            if let Some(default) = &mut param.default {
                canonicalize_expr_call_targets(default, &aliases, &imports, &local_functions);
            }
        }

        if let Some(guard) = &mut function.guard {
            canonicalize_expr_call_targets(guard, &aliases, &imports, &local_functions);
        }

        canonicalize_expr_call_targets(&mut function.body, &aliases, &imports, &local_functions);
    }
}

fn canonicalize_expr_call_targets(
    expr: &mut Expr,
    aliases: &HashMap<String, String>,
    imports: &[String],
    local_functions: &HashSet<String>,
) {
    match expr {
        Expr::Tuple { items, .. } | Expr::List { items, .. } => {
            for item in items {
                canonicalize_expr_call_targets(item, aliases, imports, local_functions);
            }
        }
        Expr::Map { entries, .. } | Expr::Keyword { entries, .. } => {
            for entry in entries {
                canonicalize_expr_call_targets(&mut entry.value, aliases, imports, local_functions);
            }
        }
        Expr::MapUpdate { base, updates, .. } => {
            canonicalize_expr_call_targets(base, aliases, imports, local_functions);
            for entry in updates {
                canonicalize_expr_call_targets(&mut entry.value, aliases, imports, local_functions);
            }
        }
        Expr::FieldAccess { base, .. } => {
            canonicalize_expr_call_targets(base, aliases, imports, local_functions);
        }
        Expr::IndexAccess { base, index, .. } => {
            canonicalize_expr_call_targets(base, aliases, imports, local_functions);
            canonicalize_expr_call_targets(index, aliases, imports, local_functions);
        }
        Expr::Call { callee, args, .. } => {
            for arg in args {
                canonicalize_expr_call_targets(arg, aliases, imports, local_functions);
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

            if imports.len() == 1 {
                *callee = format!("{}.{}", imports[0], callee);
            }
        }
        Expr::Fn { body, .. } => {
            canonicalize_expr_call_targets(body, aliases, imports, local_functions);
        }
        Expr::Invoke { callee, args, .. } => {
            canonicalize_expr_call_targets(callee, aliases, imports, local_functions);
            for arg in args {
                canonicalize_expr_call_targets(arg, aliases, imports, local_functions);
            }
        }
        Expr::Question { value, .. }
        | Expr::Group { inner: value, .. }
        | Expr::Unary { value, .. } => {
            canonicalize_expr_call_targets(value, aliases, imports, local_functions);
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            canonicalize_expr_call_targets(left, aliases, imports, local_functions);
            canonicalize_expr_call_targets(right, aliases, imports, local_functions);
        }
        Expr::Case {
            subject, branches, ..
        } => {
            canonicalize_expr_call_targets(subject, aliases, imports, local_functions);
            for branch in branches {
                if let Some(guard) = branch.guard_mut() {
                    canonicalize_expr_call_targets(guard, aliases, imports, local_functions);
                }
                canonicalize_expr_call_targets(
                    branch.body_mut(),
                    aliases,
                    imports,
                    local_functions,
                );
            }
        }
        Expr::For {
            generators,
            into,
            body,
            ..
        } => {
            for (_, generator) in generators {
                canonicalize_expr_call_targets(generator, aliases, imports, local_functions);
            }
            if let Some(into_expr) = into {
                canonicalize_expr_call_targets(into_expr, aliases, imports, local_functions);
            }
            canonicalize_expr_call_targets(body, aliases, imports, local_functions);
        }
        Expr::Try {
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            canonicalize_expr_call_targets(body, aliases, imports, local_functions);
            for branch in rescue {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr_call_targets(guard, aliases, imports, local_functions);
                }
                canonicalize_expr_call_targets(&mut branch.body, aliases, imports, local_functions);
            }
            for branch in catch {
                if let Some(guard) = branch.guard.as_mut() {
                    canonicalize_expr_call_targets(guard, aliases, imports, local_functions);
                }
                canonicalize_expr_call_targets(&mut branch.body, aliases, imports, local_functions);
            }
            if let Some(after) = after {
                canonicalize_expr_call_targets(after, aliases, imports, local_functions);
            }
        }
        Expr::Raise { error, .. } => {
            canonicalize_expr_call_targets(error, aliases, imports, local_functions);
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
                    canonicalize_expr_call_targets(expr, aliases, imports, local_functions);
                }
            }
        }
    }
}

fn is_builtin_call_target(callee: &str) -> bool {
    matches!(
        callee,
        "ok" | "err" | "tuple" | "list" | "map" | "keyword" | "protocol_dispatch" | "host_call"
    )
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
            modules.push(self.parse_module()?);
        }

        Ok(Ast { modules })
    }

    fn parse_module(&mut self) -> Result<Module, ParserError> {
        let id = self.node_ids.next_module();

        self.expect(TokenKind::Defmodule, "defmodule")?;
        let name = self.expect_ident("module name")?;
        self.expect(TokenKind::Do, "do")?;

        let mut forms = Vec::new();
        let mut attributes = Vec::new();
        let mut functions = Vec::new();

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

            return Err(self.expected("module declaration"));
        }

        self.expect(TokenKind::End, "end")?;

        let mut module = Module::with_id(id, name, forms, attributes, functions);
        canonicalize_module_call_targets(&mut module);
        Ok(module)
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
                && matches!(token.lexeme(), "alias" | "import" | "require" | "use")
        })
    }

    fn parse_module_form(&mut self) -> Result<ModuleForm, ParserError> {
        let form_name = self.expect_ident("module form")?;

        match form_name.as_str() {
            "alias" => self.parse_alias_form(),
            "import" => self.parse_named_module_form("import"),
            "require" => self.parse_named_module_form("require"),
            "use" => self.parse_named_module_form("use"),
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

    fn parse_named_module_form(&mut self, form_name: &str) -> Result<ModuleForm, ParserError> {
        let module = self.parse_module_reference("module name")?;

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "module form option")?;
            return Err(ParserError::at_current(
                format!(
                    "unsupported {form_name} option '{}'; remove options from {form_name} for now",
                    option_token.lexeme()
                ),
                Some(option_token),
            ));
        }

        let form = match form_name {
            "import" => ModuleForm::Import { module },
            "require" => ModuleForm::Require { module },
            "use" => ModuleForm::Use { module },
            _ => {
                return Err(ParserError::at_current(
                    format!("unsupported module form '{form_name}'"),
                    self.current(),
                ));
            }
        };

        Ok(form)
    }

    fn parse_module_attribute(&mut self) -> Result<ModuleAttribute, ParserError> {
        self.expect(TokenKind::At, "@")?;
        let name = self.expect_ident("attribute name")?;
        let value = self.parse_expression()?;

        Ok(ModuleAttribute { name, value })
    }

    fn parse_module_reference(&mut self, expected: &str) -> Result<String, ParserError> {
        let mut module = self.expect_ident(expected)?;

        while self.match_kind(TokenKind::Dot) {
            let segment = self.expect_ident("module name segment")?;
            module.push('.');
            module.push_str(&segment);
        }

        Ok(module)
    }

    fn parse_params(&mut self) -> Result<Vec<Parameter>, ParserError> {
        let mut params = Vec::new();
        let mut saw_default = false;

        if self.check(TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let param = self.parse_param(params.len())?;

            if saw_default && !param.has_default() {
                return Err(ParserError::at_current(
                    "default parameters must be trailing",
                    self.current(),
                ));
            }
            saw_default |= param.has_default();
            params.push(param);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(params)
    }

    fn parse_param(&mut self, index: usize) -> Result<Parameter, ParserError> {
        let (name, annotation, pattern, supports_default) =
            if self.current_starts_dynamic_param_annotation() {
                self.advance();
                let name = self.expect_ident("parameter name")?;
                (
                    name.clone(),
                    ParameterAnnotation::Dynamic,
                    Pattern::Bind { name },
                    true,
                )
            } else {
                let pattern = self.parse_pattern()?;
                let supports_default = matches!(pattern, Pattern::Bind { .. });
                let name = match &pattern {
                    Pattern::Bind { name } => name.clone(),
                    _ => format!("__arg{index}"),
                };
                (
                    name,
                    ParameterAnnotation::Inferred,
                    pattern,
                    supports_default,
                )
            };

        let default = if self.match_kind(TokenKind::BackslashBackslash) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        if default.is_some() && !supports_default {
            return Err(ParserError::at_current(
                "default values require variable parameters",
                self.current(),
            ));
        }

        match annotation {
            ParameterAnnotation::Inferred => Ok(Parameter::inferred(name, pattern, default)),
            ParameterAnnotation::Dynamic => Ok(Parameter::dynamic(name, default)),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_match_expression()
    }

    fn parse_match_expression(&mut self) -> Result<Expr, ParserError> {
        let left = self.parse_pipe_expression()?;

        if self.match_kind(TokenKind::MatchEq) {
            let right = self.parse_match_expression()?;
            return Ok(Expr::binary(
                self.node_ids.next_expr(),
                BinaryOp::Match,
                left,
                right,
            ));
        }

        Ok(left)
    }

    fn parse_pipe_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_binary_expression(0)?;

        while self.match_kind(TokenKind::PipeGt) {
            let right = self.parse_binary_expression(0)?;
            left = Expr::pipe(self.node_ids.next_expr(), left, right);
        }

        Ok(left)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> Result<Expr, ParserError> {
        let mut left = self.parse_unary_expression()?;

        while let Some((precedence, next_precedence, op)) = self.current_binary_operator() {
            if precedence < min_precedence {
                break;
            }

            self.advance();
            let right = self.parse_binary_expression(next_precedence)?;
            left = Expr::binary(self.node_ids.next_expr(), op, left, right);
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expr, ParserError> {
        if let Some(token) = self.current() {
            let unary = match token.kind() {
                TokenKind::Not => Some((UnaryOp::Not, 110)),
                TokenKind::Bang => Some((UnaryOp::Bang, 110)),
                TokenKind::Plus => Some((UnaryOp::Plus, 110)),
                TokenKind::Minus => Some((UnaryOp::Minus, 110)),
                _ => None,
            };

            if let Some((op, rbp)) = unary {
                let offset = self.advance().unwrap().span().start();
                let expr = self.parse_binary_expression(rbp)?;
                return Ok(Expr::unary(self.node_ids.next_expr(), offset, op, expr));
            }
        }

        self.parse_postfix_expression()
    }

    fn parse_postfix_expression(&mut self) -> Result<Expr, ParserError> {
        let mut expression = self.parse_atomic_expression()?;

        loop {
            if self.check(TokenKind::Question) {
                let offset = self
                    .advance()
                    .expect("question token should be available")
                    .span()
                    .start();
                expression = Expr::question(self.node_ids.next_expr(), offset, expression);
                continue;
            }

            if self.check(TokenKind::Dot) {
                if self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::LParen)
                {
                    let offset = self
                        .advance()
                        .expect("dot token should be available")
                        .span()
                        .start();
                    self.expect(TokenKind::LParen, "(")?;
                    let args = self.parse_call_args()?;
                    self.expect(TokenKind::RParen, ")")?;
                    expression = Expr::invoke(self.node_ids.next_expr(), offset, expression, args);
                    continue;
                } else if self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::Ident)
                {
                    let offset = self
                        .advance()
                        .expect("dot token should be available")
                        .span()
                        .start();
                    let label = self.expect_ident("field access label")?;
                    expression =
                        Expr::field_access(self.node_ids.next_expr(), offset, expression, label);
                    continue;
                }
            }

            if self.check(TokenKind::LBracket) {
                let has_space_before = self.index > 0
                    && self.tokens[self.index - 1].span().end()
                        < self.current().unwrap().span().start();

                if has_space_before {
                    break;
                }

                let offset = self
                    .advance()
                    .expect("lbracket token should be available")
                    .span()
                    .start();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RBracket, "]")?;
                expression =
                    Expr::index_access(self.node_ids.next_expr(), offset, expression, index);
                continue;
            }

            break;
        }

        Ok(expression)
    }

    fn parse_atomic_expression(&mut self) -> Result<Expr, ParserError> {
        if self.check(TokenKind::Lt)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Lt)
        {
            return self.parse_bitstring_literal_expression();
        }

        if self.check(TokenKind::If) {
            return self.parse_if_expression();
        }

        if self.check(TokenKind::Unless) {
            return self.parse_unless_expression();
        }

        if self.check(TokenKind::Cond) {
            return self.parse_cond_expression();
        }

        if self.check(TokenKind::With) {
            return self.parse_with_expression();
        }

        if self.check(TokenKind::For) {
            return self.parse_for_expression();
        }

        if self.check(TokenKind::Case) {
            return self.parse_case_expression();
        }

        if self.check(TokenKind::Try) {
            return self.parse_try_expression();
        }

        if self.check(TokenKind::Raise) {
            return self.parse_raise_expression();
        }

        if self.check(TokenKind::Fn) {
            return self.parse_anonymous_function_expression();
        }

        if self.check(TokenKind::Ampersand) {
            if self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::LParen)
            {
                return self.parse_capture_expression();
            }

            if self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Integer)
            {
                let offset = self
                    .advance()
                    .expect("ampersand token should be available")
                    .span()
                    .start();
                let placeholder = self
                    .expect_token(TokenKind::Integer, "capture placeholder index")?
                    .lexeme()
                    .parse::<usize>()
                    .map_err(|_| {
                        ParserError::at_current(
                            "capture placeholder index must be a positive integer",
                            self.current(),
                        )
                    })?;

                if placeholder == 0 {
                    return Err(ParserError::at_current(
                        "capture placeholder index must be >= 1",
                        self.current(),
                    ));
                }

                if let Some(current_max) = self.capture_param_max_stack.last_mut() {
                    *current_max = (*current_max).max(placeholder);
                } else {
                    return Err(ParserError::at_current(
                        "capture placeholders are only valid inside capture expressions",
                        self.current(),
                    ));
                }

                return Ok(Expr::variable(
                    self.node_ids.next_expr(),
                    offset,
                    format!("__capture{placeholder}"),
                ));
            }

            return Err(ParserError::at_current(
                "unsupported capture expression form",
                self.current(),
            ));
        }

        if self.check(TokenKind::True) {
            let token = self.advance().expect("true token should be available");
            return Ok(Expr::bool(
                self.node_ids.next_expr(),
                token.span().start(),
                true,
            ));
        }

        if self.check(TokenKind::False) {
            let token = self.advance().expect("false token should be available");
            return Ok(Expr::bool(
                self.node_ids.next_expr(),
                token.span().start(),
                false,
            ));
        }

        if self.check(TokenKind::Nil) {
            let token = self.advance().expect("nil token should be available");
            return Ok(Expr::nil(self.node_ids.next_expr(), token.span().start()));
        }

        if self.check(TokenKind::String) {
            let token = self.advance().expect("string token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::string(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::StringStart) {
            let start_token = self
                .advance()
                .expect("string start token should be available");
            let offset = start_token.span().start();
            let mut segments = Vec::new();

            loop {
                if self.check(TokenKind::StringPart) {
                    let token = self.advance().unwrap();
                    segments.push(InterpolationSegment::String {
                        value: token.lexeme().to_string(),
                    });
                } else if self.check(TokenKind::InterpolationStart) {
                    self.advance().unwrap();
                    let expr = self.parse_expression()?;
                    self.expect(
                        TokenKind::InterpolationEnd,
                        "expected '}' after interpolated expression",
                    )?;
                    segments.push(InterpolationSegment::Expr { expr });
                } else if self.check(TokenKind::StringEnd) {
                    self.advance().unwrap();
                    break;
                } else {
                    return Err(ParserError::at_current(
                        "unexpected token inside string interpolation",
                        self.peek(0),
                    ));
                }
            }

            return Ok(Expr::interpolated_string(
                self.node_ids.next_expr(),
                offset,
                segments,
            ));
        }

        if self.check(TokenKind::Float) {
            let token = self.advance().expect("float token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::float(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::Integer) {
            let token = self.advance().expect("integer token should be available");
            let offset = token.span().start();
            let value = token.lexeme().parse::<i64>().map_err(|_| {
                ParserError::at_current(
                    format!("invalid integer literal '{}'", token.lexeme()),
                    Some(token),
                )
            })?;

            return Ok(Expr::int(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::Atom) {
            let token = self.advance().expect("atom token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::atom(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::LBrace) {
            return self.parse_tuple_literal_expression();
        }

        if self.check(TokenKind::LBracket) {
            return self.parse_list_or_keyword_literal_expression();
        }

        if self.check(TokenKind::Percent) {
            return self.parse_map_literal_expression();
        }

        if self.check(TokenKind::Ident) {
            let callee_token = self
                .advance()
                .expect("identifier token should be available");
            let offset = callee_token.span().start();
            let mut callee = callee_token.lexeme().to_string();
            let mut callee_end = callee_token.span().end();

            let has_module_qualifier = callee
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());

            if self.check(TokenKind::Dot)
                && self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::Ident)
            {
                let should_parse_qualified = self
                    .peek(2)
                    .is_some_and(|token| token.kind() == TokenKind::LParen)
                    || (has_module_qualifier
                        && self.peek(2).is_some_and(|token| {
                            token_can_start_no_paren_arg(token.kind())
                                && self.peek(1).is_some_and(|function| {
                                    token.span().start() == function.span().end() + 1
                                })
                        }));

                if should_parse_qualified {
                    self.advance();
                    let function_name_token =
                        self.expect_token(TokenKind::Ident, "qualified function name")?;
                    callee_end = function_name_token.span().end();
                    callee = format!("{callee}.{}", function_name_token.lexeme());
                }
            }

            if self.match_kind(TokenKind::LParen) {
                let args = self.parse_call_args()?;
                self.expect(TokenKind::RParen, ")")?;
                return Ok(Expr::call(self.node_ids.next_expr(), offset, callee, args));
            }

            if self.current_starts_no_paren_call_arg(callee_end) {
                let args = self.parse_no_paren_call_args()?;
                return Ok(Expr::call(self.node_ids.next_expr(), offset, callee, args));
            }

            return Ok(Expr::variable(self.node_ids.next_expr(), offset, callee));
        }

        // Handle parenthesized expressions: (expr)
        if self.check(TokenKind::LParen) {
            let offset = self
                .advance()
                .expect("lparen token should be available")
                .span()
                .start();
            let inner = self.parse_expression()?;
            self.expect(TokenKind::RParen, ")")?;
            return Ok(Expr::group(self.node_ids.next_expr(), offset, inner));
        }

        Err(self.expected("expression"))
    }

    fn parse_anonymous_function_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Fn, "fn")?.span().start();
        let mut params = Vec::new();

        if !self.check(TokenKind::Arrow) {
            loop {
                params.push(self.expect_ident("anonymous function parameter")?);

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::Arrow, "->")?;
        let body = self.parse_expression()?;
        self.expect(TokenKind::End, "end")?;

        Ok(Expr::anonymous_fn(
            self.node_ids.next_expr(),
            offset,
            params,
            body,
        ))
    }

    fn parse_capture_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Ampersand, "&")?.span().start();
        self.expect(TokenKind::LParen, "(")?;

        self.capture_param_max_stack.push(0);
        let body = self.parse_expression()?;
        let max_capture_index = self
            .capture_param_max_stack
            .pop()
            .expect("capture placeholder scope should exist");

        self.expect(TokenKind::RParen, ")")?;

        if max_capture_index == 0 {
            return Err(ParserError::at_current(
                "capture expression requires at least one placeholder",
                self.current(),
            ));
        }

        let params = (1..=max_capture_index)
            .map(|index| format!("__capture{index}"))
            .collect::<Vec<_>>();

        Ok(Expr::anonymous_fn(
            self.node_ids.next_expr(),
            offset,
            params,
            body,
        ))
    }

    fn parse_if_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::If, "if")?.span().start();
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Do, "do")?;

        let then_body = self.parse_expression()?;
        let else_body = if self.match_kind(TokenKind::Else) {
            self.parse_expression()?
        } else {
            Expr::nil(self.node_ids.next_expr(), offset)
        };

        self.expect(TokenKind::End, "end")?;

        Ok(self.lower_guarded_control_case(offset, condition, then_body, else_body))
    }

    fn parse_unless_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self
            .expect_token(TokenKind::Unless, "unless")?
            .span()
            .start();
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Do, "do")?;

        let then_body = self.parse_expression()?;
        let else_body = if self.match_kind(TokenKind::Else) {
            self.parse_expression()?
        } else {
            Expr::nil(self.node_ids.next_expr(), offset)
        };

        self.expect(TokenKind::End, "end")?;

        Ok(self.lower_guarded_control_case(offset, condition, else_body, then_body))
    }

    fn parse_cond_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Cond, "cond")?.span().start();
        self.expect(TokenKind::Do, "do")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("cond branch"));
            }

            let condition = self.parse_expression()?;
            self.expect(TokenKind::Arrow, "->")?;
            let body = self.parse_expression()?;
            let guard = self.lower_truthy_guard(condition);
            branches.push(CaseBranch::new(Pattern::Wildcard, Some(guard), body));
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::nil(self.node_ids.next_expr(), offset),
            branches,
        ))
    }

    fn parse_with_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::With, "with")?.span().start();
        let mut clauses = Vec::new();

        loop {
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::LeftArrow, "<-")?;
            let value = self.parse_expression()?;
            clauses.push((pattern, value));

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_expression()?;

        let else_branches = if self.match_kind(TokenKind::Else) {
            let mut branches = Vec::new();

            while !self.check(TokenKind::End) {
                if self.is_at_end() {
                    return Err(self.expected("with else branch"));
                }

                branches.push(self.parse_case_branch()?);
            }

            branches
        } else {
            Vec::new()
        };

        self.expect(TokenKind::End, "end")?;

        Ok(self.lower_with_expression(offset, clauses, body, else_branches))
    }

    fn parse_for_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::For, "for")?.span().start();

        let mut generators = Vec::new();
        let mut into_expr = None;
        loop {
            if self.check(TokenKind::Ident)
                && self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::Colon)
            {
                let option_token = self.expect_token(TokenKind::Ident, "for option")?;
                self.expect(TokenKind::Colon, ":")?;

                if option_token.lexeme() == "into" {
                    into_expr = Some(self.parse_expression()?);
                } else {
                    return Err(ParserError::at_current(
                        format!(
                            "unsupported for option '{}'; remove options from for for now",
                            option_token.lexeme()
                        ),
                        Some(option_token),
                    ));
                }

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }
                break;
            }

            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::LeftArrow, "<-")?;
            let generator = self.parse_expression()?;
            generators.push((pattern, generator));

            if self.match_kind(TokenKind::Comma) {
                continue;
            }
            break;
        }

        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_expression()?;
        self.expect(TokenKind::End, "end")?;

        Ok(Expr::for_comprehension(
            self.node_ids.next_expr(),
            offset,
            generators,
            into_expr,
            body,
        ))
    }

    fn lower_with_expression(
        &mut self,
        offset: usize,
        clauses: Vec<(Pattern, Expr)>,
        body: Expr,
        else_branches: Vec<CaseBranch>,
    ) -> Expr {
        let mut lowered = body;

        for (pattern, value) in clauses.into_iter().rev() {
            let failure_binding = "__tonic_with_failure".to_string();
            let failure_handler = if else_branches.is_empty() {
                Expr::variable(self.node_ids.next_expr(), offset, failure_binding.clone())
            } else {
                Expr::case(
                    self.node_ids.next_expr(),
                    offset,
                    Expr::variable(self.node_ids.next_expr(), offset, failure_binding.clone()),
                    else_branches.clone(),
                )
            };

            lowered = Expr::case(
                self.node_ids.next_expr(),
                value.offset(),
                value,
                vec![
                    CaseBranch::new(pattern, None, lowered),
                    CaseBranch::new(
                        Pattern::Bind {
                            name: failure_binding,
                        },
                        None,
                        failure_handler,
                    ),
                ],
            );
        }

        lowered
    }

    fn lower_guarded_control_case(
        &mut self,
        offset: usize,
        condition: Expr,
        truthy_body: Expr,
        fallback_body: Expr,
    ) -> Expr {
        let guard = self.lower_truthy_guard(condition);

        Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::nil(self.node_ids.next_expr(), offset),
            vec![
                CaseBranch::new(Pattern::Wildcard, Some(guard), truthy_body),
                CaseBranch::new(Pattern::Wildcard, None, fallback_body),
            ],
        )
    }

    fn lower_truthy_guard(&mut self, condition: Expr) -> Expr {
        let offset = condition.offset();
        let first_bang = Expr::unary(self.node_ids.next_expr(), offset, UnaryOp::Bang, condition);
        Expr::unary(self.node_ids.next_expr(), offset, UnaryOp::Bang, first_bang)
    }

    fn parse_bitstring_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Lt, "<")?.span().start();
        self.expect(TokenKind::Lt, "<")?;

        let mut items = Vec::new();
        if !(self.check(TokenKind::Gt)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Gt))
        {
            loop {
                items.push(self.parse_atomic_expression()?);

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::Gt, ">")?;
        self.expect(TokenKind::Gt, ">")?;

        Ok(Expr::list(self.node_ids.next_expr(), offset, items))
    }

    fn parse_tuple_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::LBrace, "{")?.span().start();
        let items = self.parse_expression_items(TokenKind::RBrace, "}")?;
        Ok(Expr::tuple(self.node_ids.next_expr(), offset, items))
    }

    fn parse_list_or_keyword_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::LBracket, "[")?.span().start();

        if self.check(TokenKind::RBracket) {
            self.advance();
            return Ok(Expr::list(self.node_ids.next_expr(), offset, Vec::new()));
        }

        if self.starts_keyword_literal_entry() {
            let entries = self.parse_label_entries(TokenKind::RBracket, "keyword key")?;
            return Ok(Expr::keyword(self.node_ids.next_expr(), offset, entries));
        }

        let items = self.parse_expression_items(TokenKind::RBracket, "]")?;
        Ok(Expr::list(self.node_ids.next_expr(), offset, items))
    }

    fn parse_map_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Percent, "%")?.span().start();
        self.expect(TokenKind::LBrace, "{")?;

        if self.match_kind(TokenKind::RBrace) {
            return Ok(Expr::map(self.node_ids.next_expr(), offset, Vec::new()));
        }

        if self.starts_keyword_literal_entry() {
            let entries = self.parse_label_entries(TokenKind::RBrace, "map key")?;
            return Ok(Expr::map(self.node_ids.next_expr(), offset, entries));
        }

        let base = self.parse_expression()?;

        if self.match_kind(TokenKind::Pipe) {
            let entries = self.parse_label_entries(TokenKind::RBrace, "map update key")?;
            return Ok(Expr::map_update(
                self.node_ids.next_expr(),
                offset,
                base,
                entries,
            ));
        }

        Err(ParserError::at_current(
            "expected map update pipe `|` or label entry",
            self.current(),
        ))
    }

    fn starts_keyword_literal_entry(&self) -> bool {
        self.check(TokenKind::Ident)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Colon)
    }

    fn parse_label_entries(
        &mut self,
        closing: TokenKind,
        expected_key: &str,
    ) -> Result<Vec<LabelExprEntry>, ParserError> {
        let mut entries = Vec::new();

        loop {
            let key = self.expect_ident(expected_key)?;
            self.expect(TokenKind::Colon, ":")?;
            let value = self.parse_expression()?;
            entries.push(LabelExprEntry { key, value });

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(closing, "literal terminator")?;
        Ok(entries)
    }

    fn parse_expression_items(
        &mut self,
        closing: TokenKind,
        expected_closing: &str,
    ) -> Result<Vec<Expr>, ParserError> {
        let mut items = Vec::new();

        if self.check(closing) {
            self.advance();
            return Ok(items);
        }

        loop {
            items.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(closing, expected_closing)?;
        Ok(items)
    }

    fn parse_case_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Case, "case")?.span().start();
        let subject = self.parse_expression()?;
        self.expect(TokenKind::Do, "do")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("case branch"));
            }

            branches.push(self.parse_case_branch()?);
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            subject,
            branches,
        ))
    }

    fn parse_case_branch(&mut self) -> Result<CaseBranch, ParserError> {
        let pattern = self.parse_pattern()?;
        let guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Arrow, "->")?;
        let body = self.parse_expression()?;

        Ok(CaseBranch::new(pattern, guard, body))
    }

    fn parse_try_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Try, "try")?.span().start();
        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_expression()?;

        let mut rescue = Vec::new();
        if self.match_kind(TokenKind::Rescue) {
            while !self.check(TokenKind::Catch)
                && !self.check(TokenKind::After)
                && !self.check(TokenKind::End)
            {
                if self.is_at_end() {
                    return Err(self.expected("rescue branch, catch branch, after block, or end"));
                }
                rescue.push(self.parse_case_branch()?);
            }
        }

        let mut catch = Vec::new();
        if self.match_kind(TokenKind::Catch) {
            while !self.check(TokenKind::After) && !self.check(TokenKind::End) {
                if self.is_at_end() {
                    return Err(self.expected("catch branch, after block, or end"));
                }
                catch.push(self.parse_case_branch()?);
            }
        }

        let after = if self.match_kind(TokenKind::After) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        if rescue.is_empty() && catch.is_empty() && after.is_none() {
            return Err(ParserError::at_current(
                "try must be followed by rescue, catch, or after",
                Some(&self.tokens[self.index - 1]),
            ));
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Expr::try_expr(
            self.node_ids.next_expr(),
            offset,
            body,
            rescue,
            catch,
            after,
        ))
    }

    fn parse_raise_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Raise, "raise")?.span().start();

        let has_parens = self.match_kind(TokenKind::LParen);
        let error = self.parse_expression()?;
        if has_parens {
            self.expect(TokenKind::RParen, ")")?;
        }

        Ok(Expr::raise(self.node_ids.next_expr(), offset, error))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParserError> {
        if self.match_kind(TokenKind::Caret) {
            let name = self.expect_ident("pinned variable")?;
            return Ok(Pattern::Pin { name });
        }

        if self.check(TokenKind::Integer) {
            let token = self.advance().expect("integer token should be available");
            let value = token.lexeme().parse::<i64>().map_err(|_| {
                ParserError::at_current(
                    format!("invalid integer literal '{}'", token.lexeme()),
                    Some(token),
                )
            })?;
            return Ok(Pattern::Integer { value });
        }

        if self.check(TokenKind::True) {
            self.advance();
            return Ok(Pattern::Bool { value: true });
        }

        if self.check(TokenKind::False) {
            self.advance();
            return Ok(Pattern::Bool { value: false });
        }

        if self.check(TokenKind::Nil) {
            self.advance();
            return Ok(Pattern::Nil);
        }

        if self.check(TokenKind::String) {
            let value = self
                .advance()
                .expect("string token should be available")
                .lexeme()
                .to_string();
            return Ok(Pattern::String { value });
        }

        if self.match_kind(TokenKind::LBrace) {
            let (items, tail) = self.parse_pattern_items(TokenKind::RBrace)?;
            if tail.is_some() {
                return Err(ParserError::at_current(
                    "tuple patterns do not support tail syntax",
                    self.current(),
                ));
            }
            return Ok(Pattern::Tuple { items });
        }

        if self.match_kind(TokenKind::LBracket) {
            let (items, tail) = self.parse_pattern_items(TokenKind::RBracket)?;
            return Ok(Pattern::List { items, tail });
        }

        if self.match_kind(TokenKind::Percent) {
            return self.parse_map_pattern();
        }

        if self.check(TokenKind::Atom) {
            let value = self
                .advance()
                .expect("atom token should be available")
                .lexeme()
                .to_string();
            return Ok(Pattern::Atom { value });
        }

        if self.check(TokenKind::Ident) {
            let name = self
                .advance()
                .expect("identifier token should be available")
                .lexeme()
                .to_string();

            if name == "_" {
                return Ok(Pattern::Wildcard);
            }

            return Ok(Pattern::Bind { name });
        }

        Err(self.expected("pattern"))
    }

    fn parse_pattern_items(
        &mut self,
        closing: TokenKind,
    ) -> Result<(Vec<Pattern>, Option<Box<Pattern>>), ParserError> {
        let mut items = Vec::new();
        let mut tail = None;

        if self.check(closing) {
            self.advance();
            return Ok((items, tail));
        }

        loop {
            items.push(self.parse_pattern()?);

            if self.match_kind(TokenKind::Pipe) {
                tail = Some(Box::new(self.parse_pattern()?));
                break;
            }

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(closing, "pattern terminator")?;
        Ok((items, tail))
    }

    fn parse_map_pattern(&mut self) -> Result<Pattern, ParserError> {
        self.expect(TokenKind::LBrace, "{")?;

        let mut entries = Vec::new();
        if !self.check(TokenKind::RBrace) {
            loop {
                let (key, value) = if self.check(TokenKind::Ident)
                    && self
                        .peek(1)
                        .is_some_and(|token| token.kind() == TokenKind::Colon)
                {
                    let key = Pattern::Atom {
                        value: self.expect_ident("map pattern key")?,
                    };
                    self.expect(TokenKind::Colon, ":")?;
                    let value = self.parse_pattern()?;
                    (key, value)
                } else {
                    let key = self.parse_pattern()?;
                    self.expect(TokenKind::Arrow, "->")?;
                    let value = self.parse_pattern()?;
                    (key, value)
                };

                entries.push(MapPatternEntry { key, value });

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::RBrace, "}")?;

        Ok(Pattern::Map { entries })
    }

    fn current_binary_operator(&self) -> Option<(u8, u8, BinaryOp)> {
        self.current().and_then(|token| match token.kind() {
            TokenKind::Star => Some((100, 101, BinaryOp::Mul)),
            TokenKind::Slash => Some((100, 101, BinaryOp::Div)),
            TokenKind::Plus => Some((90, 91, BinaryOp::Plus)),
            TokenKind::Minus => Some((90, 91, BinaryOp::Minus)),
            TokenKind::LessGreater => Some((80, 80, BinaryOp::Concat)),
            TokenKind::PlusPlus => Some((80, 80, BinaryOp::PlusPlus)),
            TokenKind::MinusMinus => Some((80, 80, BinaryOp::MinusMinus)),
            TokenKind::DotDot => Some((80, 80, BinaryOp::Range)),
            TokenKind::In => Some((70, 71, BinaryOp::In)),
            TokenKind::EqEq => Some((60, 61, BinaryOp::Eq)),
            TokenKind::BangEq => Some((60, 61, BinaryOp::NotEq)),
            TokenKind::Lt => Some((60, 61, BinaryOp::Lt)),
            TokenKind::LtEq => Some((60, 61, BinaryOp::Lte)),
            TokenKind::Gt => Some((60, 61, BinaryOp::Gt)),
            TokenKind::GtEq => Some((60, 61, BinaryOp::Gte)),
            TokenKind::AndAnd => Some((50, 51, BinaryOp::AndAnd)),
            TokenKind::And => Some((50, 51, BinaryOp::And)),
            TokenKind::OrOr => Some((40, 41, BinaryOp::OrOr)),
            TokenKind::Or => Some((40, 41, BinaryOp::Or)),
            _ => None,
        })
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParserError> {
        let mut args = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(args)
    }

    fn parse_no_paren_call_args(&mut self) -> Result<Vec<Expr>, ParserError> {
        let mut args = Vec::new();

        loop {
            args.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(args)
    }

    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<(), ParserError> {
        self.expect_token(kind, expected).map(|_| ())
    }

    fn expect_token(&mut self, kind: TokenKind, expected: &str) -> Result<&'a Token, ParserError> {
        if self.check(kind) {
            Ok(self.advance().expect("expected token should be available"))
        } else {
            Err(self.expected(expected))
        }
    }

    fn expect_ident(&mut self, expected: &str) -> Result<String, ParserError> {
        if !self.check(TokenKind::Ident) {
            return Err(self.expected(expected));
        }

        Ok(self
            .advance()
            .expect("identifier token should be available")
            .lexeme()
            .to_string())
    }

    fn expected(&self, expected: &str) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_current(
            format!("expected {expected}, found {found}"),
            self.current(),
        )
    }

    fn current_starts_dynamic_param_annotation(&self) -> bool {
        let Some(current) = self.current() else {
            return false;
        };

        if current.kind() != TokenKind::Ident || current.lexeme() != "dynamic" {
            return false;
        }

        self.peek(1)
            .map(|next| next.kind() == TokenKind::Ident)
            .unwrap_or(false)
    }

    fn current_starts_no_paren_call_arg(&self, callee_end: usize) -> bool {
        let Some(current) = self.current() else {
            return false;
        };

        if current.span().start() != callee_end + 1 {
            return false;
        }

        if current.kind() == TokenKind::Ident && current.lexeme() == "_" {
            return false;
        }

        token_can_start_no_paren_arg(current.kind())
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current()
            .map(|token| token.kind() == kind)
            .unwrap_or(false)
    }

    fn match_kind(&mut self, kind: TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.index += 1;
        true
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.current()?;
        self.index += 1;
        Some(token)
    }

    fn current(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    fn peek(&self, distance: usize) -> Option<&'a Token> {
        self.tokens.get(self.index + distance)
    }

    fn is_at_end(&self) -> bool {
        self.check(TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_ast, Expr};
    use crate::lexer::scan_tokens;

    #[test]
    fn parse_ast_supports_single_module_with_two_functions() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(ast.modules.len(), 1);
        assert_eq!(ast.modules[0].name, "Math");
        assert_eq!(ast.modules[0].functions.len(), 2);
        assert_eq!(ast.modules[0].functions[0].name, "one");
        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"int","value":1})
        );
        assert_eq!(ast.modules[0].functions[1].name, "two");
        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[1].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"call","callee":"one","args":[]})
        );
    }

    #[test]
    fn parse_ast_supports_nested_calls_with_plus_precedence() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def compute() do\n    combine(1, 2) + wrap(inner(3 + 4))\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"binary",
                "op":"plus",
                "left":{
                    "kind":"call",
                    "callee":"combine",
                    "args":[
                        {"kind":"int","value":1},
                        {"kind":"int","value":2}
                    ]
                },
                "right":{
                    "kind":"call",
                    "callee":"wrap",
                    "args":[
                        {
                            "kind":"call",
                            "callee":"inner",
                            "args":[
                                {
                                    "kind":"binary",
                                    "op":"plus",
                                    "left":{"kind":"int","value":3},
                                    "right":{"kind":"int","value":4}
                                }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn parse_ast_supports_module_qualified_calls() {
        let tokens =
            scan_tokens("defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n")
                .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"call","callee":"Math.helper","args":[]})
        );
    }

    #[test]
    fn parse_ast_supports_no_paren_calls() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper 7\n  end\nend\n",
        )
        .expect("scanner should tokenize no-paren call fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[1].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"call",
                "callee":"helper",
                "args":[{"kind":"int","value":7}]
            })
        );
    }

    #[test]
    fn parse_ast_supports_no_paren_module_qualified_calls() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def one(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.one 7\n  end\nend\n",
        )
        .expect("scanner should tokenize no-paren qualified call fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[1].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"call",
                "callee":"Math.one",
                "args":[{"kind":"int","value":7}]
            })
        );
    }

    #[test]
    fn parse_ast_supports_try_as_no_paren_call_arg() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper try do\n      :ok\n    rescue\n      _ -> :err\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize try no-paren call fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert!(
            matches!(ast.modules[0].functions[1].body, Expr::Call { .. }),
            "outermost expr should be call"
        );
    }

    #[test]
    fn parse_ast_supports_raise_as_no_paren_call_arg() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper raise \"boom\"\n  end\nend\n",
        )
        .expect("scanner should tokenize raise no-paren call fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert!(
            matches!(ast.modules[0].functions[1].body, Expr::Call { .. }),
            "outermost expr should be call"
        );
    }

    #[test]
    fn parse_ast_supports_postfix_question_operator() {
        let tokens = scan_tokens("defmodule Demo do\n  def run() do\n    value()?\n  end\nend\n")
            .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"question",
                "value":{"kind":"call","callee":"value","args":[]}
            })
        );
    }

    #[test]
    fn parse_ast_supports_bitstring_literals_as_list_values() {
        let tokens =
            scan_tokens("defmodule Demo do\n  def run() do\n    <<1, 2, 3>>\n  end\nend\n")
                .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"list",
                "items":[
                    {"kind":"int","value":1},
                    {"kind":"int","value":2},
                    {"kind":"int","value":3}
                ]
            })
        );
    }

    #[test]
    fn parse_ast_supports_case_patterns() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      [head, tail] -> 2\n      %{} -> 3\n      _ -> 4\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"case",
                "subject":{"kind":"call","callee":"input","args":[]},
                "branches":[
                    {
                        "pattern":{
                            "kind":"tuple",
                            "items":[
                                {"kind":"atom","value":"ok"},
                                {"kind":"bind","name":"value"}
                            ]
                        },
                        "body":{"kind":"int","value":1}
                    },
                    {
                        "pattern":{
                            "kind":"list",
                            "items":[
                                {"kind":"bind","name":"head"},
                                {"kind":"bind","name":"tail"}
                            ]
                        },
                        "body":{"kind":"int","value":2}
                    },
                    {
                        "pattern":{"kind":"map","entries":[]},
                        "body":{"kind":"int","value":3}
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "body":{"kind":"int","value":4}
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_ast_supports_list_cons_patterns() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      [head | tail] -> head\n      _ -> 0\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"case",
                "subject":{"kind":"call","callee":"input","args":[]},
                "branches":[
                    {
                        "pattern":{
                            "kind":"list",
                            "items":[{"kind":"bind","name":"head"}],
                            "tail":{"kind":"bind","name":"tail"}
                        },
                        "body":{"kind":"variable","name":"head"}
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "body":{"kind":"int","value":0}
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_ast_supports_map_colon_patterns() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      %{ok: value} -> value\n      _ -> 0\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"case",
                "subject":{"kind":"call","callee":"input","args":[]},
                "branches":[
                    {
                        "pattern":{
                            "kind":"map",
                            "entries":[
                                {
                                    "key":{"kind":"atom","value":"ok"},
                                    "value":{"kind":"bind","name":"value"}
                                }
                            ]
                        },
                        "body":{"kind":"variable","name":"value"}
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "body":{"kind":"int","value":0}
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_ast_supports_pin_patterns_case_guards_and_match_operator() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case list(7, 8) do\n      [^value, tail] when tail == 8 -> value = tail\n      _ -> 0\n    end\n  end\n\n  def value() do\n    7\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"case",
                "subject":{"kind":"call","callee":"list","args":[{"kind":"int","value":7},{"kind":"int","value":8}]},
                "branches":[
                    {
                        "pattern":{
                            "kind":"list",
                            "items":[
                                {"kind":"pin","name":"value"},
                                {"kind":"bind","name":"tail"}
                            ]
                        },
                        "guard":{
                            "kind":"binary",
                            "op":"eq",
                            "left":{"kind":"variable","name":"tail"},
                            "right":{"kind":"int","value":8}
                        },
                        "body":{
                            "kind":"binary",
                            "op":"match",
                            "left":{"kind":"variable","name":"value"},
                            "right":{"kind":"variable","name":"tail"}
                        }
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "body":{"kind":"int","value":0}
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_ast_exposes_normalized_case_branch_head_and_body() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      _ -> 2\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");
        let Expr::Case { branches, .. } = &ast.modules[0].functions[0].body else {
            panic!("expected case expression body");
        };

        assert_eq!(branches.len(), 2);
        assert_eq!(
            serde_json::to_value(branches[0].head()).expect("branch head should serialize"),
            serde_json::json!({
                "kind":"tuple",
                "items":[
                    {"kind":"atom","value":"ok"},
                    {"kind":"bind","name":"value"}
                ]
            })
        );
        assert_eq!(
            serde_json::to_value(branches[0].body()).expect("branch body should serialize"),
            serde_json::json!({"kind":"int","value":1})
        );
    }

    #[test]
    fn parse_ast_supports_function_head_patterns_defaults_and_private_defs() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def classify({:ok, value}) do\n    value\n  end\n\n  defp add(value, inc \\\\ 2) do\n    value + inc\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(ast.modules[0].functions[0].params[0].name(), "__arg0");
        assert_eq!(
            serde_json::to_value(ast.modules[0].functions[0].params[0].pattern())
                .expect("pattern should serialize"),
            serde_json::json!({
                "kind":"tuple",
                "items":[
                    {"kind":"atom","value":"ok"},
                    {"kind":"bind","name":"value"}
                ]
            })
        );
        assert!(ast.modules[0].functions[1].is_private());
        assert!(ast.modules[0].functions[1].params[1].default().is_some());
    }

    #[test]
    fn parse_ast_supports_anonymous_functions_capture_and_invocation() {
        let tokens =
            scan_tokens("defmodule Demo do\n  def run() do\n    (&(&1 + 1)).(2)\n  end\nend\n")
                .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"invoke",
                "callee":{
                    "kind":"group",
                    "inner":{
                        "kind":"fn",
                        "params":["__capture1"],
                        "body":{
                            "kind":"binary",
                            "op":"plus",
                            "left":{"kind":"variable","name":"__capture1"},
                            "right":{"kind":"int","value":1}
                        }
                    }
                },
                "args":[{"kind":"int","value":2}]
            })
        );
    }

    #[test]
    fn parse_ast_supports_if_unless_cond_and_with_forms() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def pick(flag) do\n    if flag do\n      1\n    else\n      0\n    end\n  end\n\n  def reject(flag) do\n    unless flag do\n      2\n    else\n      3\n    end\n  end\n\n  def route(value) do\n    cond do\n      value > 2 -> 4\n      true -> 5\n    end\n  end\n\n  def chain() do\n    with [left, right] <- list(1, 2),\n         total <- left + right do\n      total\n    else\n      _ -> 0\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");
        let functions = &ast.modules[0].functions;

        assert!(matches!(functions[0].body, Expr::Case { .. }));
        assert!(matches!(functions[1].body, Expr::Case { .. }));
        assert!(matches!(functions[2].body, Expr::Case { .. }));
        assert!(matches!(functions[3].body, Expr::Case { .. }));
    }

    #[test]
    fn parse_ast_supports_for_comprehensions() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def run() do\n    for x <- list(1, 2, 3) do\n      x + 1\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"for",
                "into": null,
                "generators":[
                    [
                        {"kind":"bind","name":"x"},
                        {
                            "kind":"call",
                            "callee":"list",
                            "args":[
                                {"kind":"int","value":1},
                                {"kind":"int","value":2},
                                {"kind":"int","value":3}
                            ]
                        }
                    ]
                ],
                "body":{
                    "kind":"binary",
                    "op":"plus",
                    "left":{"kind":"variable","name":"x"},
                    "right":{"kind":"int","value":1}
                }
            })
        );
    }

    #[test]
    fn parse_ast_supports_for_with_multiple_generators() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), y <- list(3, 4) do\n      x + y\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should not reject multi-generator for forms");

        let body_json = serde_json::to_value(&ast.modules[0].functions[0].body).unwrap();
        assert_eq!(body_json["kind"], "for");
        assert_eq!(body_json["generators"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_ast_rejects_for_with_options() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), reduce: [] do\n      x\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let error = parse_ast(&tokens).expect_err("parser should reject unsupported for options");

        assert_eq!(
            error.to_string(),
            "unsupported for option 'reduce'; remove options from for for now at offset 58"
        );
    }

    #[test]
    fn parse_ast_rejects_non_trailing_default_params() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  def add(value \\\\ 1, other) do\n    value + other\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let error =
            parse_ast(&tokens).expect_err("parser should reject non-trailing default params");

        assert!(
            error
                .to_string()
                .starts_with("default parameters must be trailing at offset"),
            "unexpected parser error: {error}"
        );
    }

    #[test]
    fn parse_ast_assigns_stable_node_ids() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let first = parse_ast(&tokens).expect("parser should produce ast");
        let second = parse_ast(&tokens).expect("parser should produce ast");

        let first_ids = collect_node_ids(&first);
        let second_ids = collect_node_ids(&second);

        assert_eq!(
            first_ids,
            [
                "module-0001",
                "function-0002",
                "expr-0003",
                "function-0004",
                "expr-0005",
            ]
        );
        assert_eq!(first_ids, second_ids);

        let unique_count = first_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();

        assert_eq!(unique_count, first_ids.len());
    }

    #[test]
    fn parse_ast_supports_module_forms_and_attributes() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  alias Math, as: M\n  import Math\n  require Logger\n  use Feature\n  @moduledoc \"demo module\"\n  @doc \"run docs\"\n  @answer 5\n\n  def run() do\n    M.helper() + helper()\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].forms).expect("module forms should serialize"),
            serde_json::json!([
                {"kind":"alias","module":"Math","as":"M"},
                {"kind":"import","module":"Math"},
                {"kind":"require","module":"Logger"},
                {"kind":"use","module":"Feature"}
            ])
        );
        assert_eq!(
            serde_json::to_value(&ast.modules[0].attributes)
                .expect("module attributes should serialize"),
            serde_json::json!([
                {"name":"moduledoc","value":{"kind":"string","value":"demo module"}},
                {"name":"doc","value":{"kind":"string","value":"run docs"}},
                {"name":"answer","value":{"kind":"int","value":5}}
            ])
        );
        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"binary",
                "op":"plus",
                "left":{"kind":"call","callee":"Math.helper","args":[]},
                "right":{"kind":"call","callee":"Math.helper","args":[]}
            })
        );
    }

    #[test]
    fn parse_ast_rejects_unsupported_alias_options() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  alias Math, via: M\n\n  def run() do\n    1\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let error = parse_ast(&tokens).expect_err("parser should reject unsupported alias options");

        assert_eq!(
            error.to_string(),
            "unsupported alias option 'via'; supported syntax: alias Module, as: Name at offset 32"
        );
    }

    #[test]
    fn parse_ast_rejects_unsupported_import_options_with_actionable_hint() {
        let tokens = scan_tokens(
            "defmodule Demo do\n  import Math, only: [helper: 1]\n\n  def run() do\n    helper(1)\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let error =
            parse_ast(&tokens).expect_err("parser should reject unsupported import options");

        assert_eq!(
            error.to_string(),
            "unsupported import option 'only'; remove options from import for now at offset 33"
        );
    }

    #[test]
    fn parse_ast_reports_missing_module_end() {
        let tokens = scan_tokens("defmodule Broken do\n  def one() do\n    1\n  end\n")
            .expect("scanner should tokenize parser fixture");

        let error = parse_ast(&tokens).expect_err("parser should reject missing end");

        assert!(
            error
                .to_string()
                .starts_with("expected module declaration, found EOF"),
            "unexpected parser error: {error}"
        );
    }

    fn collect_node_ids(ast: &super::Ast) -> Vec<String> {
        let mut ids = Vec::new();

        for module in &ast.modules {
            ids.push(module.id.0.clone());

            for function in &module.functions {
                ids.push(function.id.0.clone());
                for param in &function.params {
                    if let Some(default) = param.default() {
                        collect_expr_ids(default, &mut ids);
                    }
                }
                if let Some(guard) = function.guard() {
                    collect_expr_ids(guard, &mut ids);
                }
                collect_expr_ids(&function.body, &mut ids);
            }
        }

        ids
    }

    fn collect_expr_ids(expr: &Expr, ids: &mut Vec<String>) {
        match expr {
            Expr::Int { id, .. }
            | Expr::Float { id, .. }
            | Expr::Bool { id, .. }
            | Expr::Nil { id, .. }
            | Expr::String { id, .. } => ids.push(id.0.clone()),
            Expr::InterpolatedString { id, segments, .. } => {
                ids.push(id.0.clone());
                for segment in segments {
                    if let crate::parser::InterpolationSegment::Expr { expr } = segment {
                        collect_expr_ids(expr, ids);
                    }
                }
            }
            Expr::Tuple { id, items, .. } | Expr::List { id, items, .. } => {
                ids.push(id.0.clone());

                for item in items {
                    collect_expr_ids(item, ids);
                }
            }
            Expr::Map { id, entries, .. } | Expr::Keyword { id, entries, .. } => {
                ids.push(id.0.clone());

                for entry in entries {
                    collect_expr_ids(&entry.value, ids);
                }
            }
            Expr::MapUpdate {
                id, base, updates, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(base, ids);
                for entry in updates {
                    collect_expr_ids(&entry.value, ids);
                }
            }
            Expr::FieldAccess { id, base, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(base, ids);
            }
            Expr::IndexAccess {
                id, base, index, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(base, ids);
                collect_expr_ids(index, ids);
            }
            Expr::Call { id, args, .. } => {
                ids.push(id.0.clone());

                for arg in args {
                    collect_expr_ids(arg, ids);
                }
            }
            Expr::Fn { id, body, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(body, ids);
            }
            Expr::Invoke {
                id, callee, args, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(callee, ids);
                for arg in args {
                    collect_expr_ids(arg, ids);
                }
            }
            Expr::Question { id, value, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(value, ids);
            }
            Expr::Binary {
                id, left, right, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(left, ids);
                collect_expr_ids(right, ids);
            }
            Expr::Unary { id, value, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(value, ids);
            }
            Expr::Pipe {
                id, left, right, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(left, ids);
                collect_expr_ids(right, ids);
            }
            Expr::Case {
                id,
                subject,
                branches,
                ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(subject, ids);

                for branch in branches {
                    if let Some(guard) = branch.guard() {
                        collect_expr_ids(guard, ids);
                    }
                    collect_expr_ids(branch.body(), ids);
                }
            }
            Expr::For {
                id,
                generators,
                into,
                body,
                ..
            } => {
                ids.push(id.0.clone());
                for (_, generator) in generators {
                    collect_expr_ids(generator, ids);
                }
                if let Some(into_expr) = into {
                    collect_expr_ids(into_expr, ids);
                }
                collect_expr_ids(body, ids);
            }
            Expr::Group { id, inner, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(inner, ids);
            }
            Expr::Try {
                id,
                body,
                rescue,
                catch,
                after,
                ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(body, ids);
                for branch in rescue {
                    if let Some(guard) = &branch.guard {
                        collect_expr_ids(guard, ids);
                    }
                    collect_expr_ids(&branch.body, ids);
                }
                for branch in catch {
                    if let Some(guard) = &branch.guard {
                        collect_expr_ids(guard, ids);
                    }
                    collect_expr_ids(&branch.body, ids);
                }
                if let Some(after) = after {
                    collect_expr_ids(after, ids);
                }
            }
            Expr::Raise { id, error, .. } => {
                ids.push(id.0.clone());
                collect_expr_ids(error, ids);
            }
            Expr::Variable { id, .. } | Expr::Atom { id, .. } => {
                ids.push(id.0.clone());
            }
        }
    }
}
