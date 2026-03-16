use super::*;
use serde::Serialize;

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
