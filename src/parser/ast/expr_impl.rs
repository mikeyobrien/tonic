use super::*;

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
