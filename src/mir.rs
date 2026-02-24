mod lower;
#[cfg(test)]
mod tests;

use crate::ir::{IrCallTarget, IrOp, IrPattern, IrProgram};
use serde::{Deserialize, Serialize};
use std::fmt;

pub(crate) fn lower_ir_to_mir(ir: &IrProgram) -> Result<MirProgram, MirLoweringError> {
    lower::lower_ir_to_mir_impl(ir)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MirProgram {
    pub(crate) functions: Vec<MirFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MirFunction {
    pub(crate) name: String,
    pub(crate) params: Vec<MirTypedName>,
    pub(crate) entry_block: u32,
    pub(crate) blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MirTypedName {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) value_type: MirType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MirBlock {
    pub(crate) id: u32,
    pub(crate) args: Vec<MirTypedName>,
    pub(crate) instructions: Vec<MirInstruction>,
    pub(crate) terminator: MirTerminator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MirType {
    Int,
    Float,
    Bool,
    Nil,
    String,
    Atom,
    Result,
    Closure,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum MirInstruction {
    ConstInt {
        dest: u32,
        value: i64,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    ConstFloat {
        dest: u32,
        value: String,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    ConstBool {
        dest: u32,
        value: bool,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    ConstNil {
        dest: u32,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    ConstString {
        dest: u32,
        value: String,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    ConstAtom {
        dest: u32,
        value: String,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    LoadVariable {
        dest: u32,
        name: String,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    Unary {
        dest: u32,
        kind: MirUnaryKind,
        input: u32,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    Binary {
        dest: u32,
        kind: MirBinaryKind,
        left: u32,
        right: u32,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    Call {
        dest: u32,
        callee: IrCallTarget,
        args: Vec<u32>,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    CallValue {
        dest: u32,
        callee: u32,
        args: Vec<u32>,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    MakeClosure {
        dest: u32,
        params: Vec<String>,
        ops: Vec<IrOp>,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    Question {
        dest: u32,
        input: u32,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    MatchPattern {
        dest: u32,
        input: u32,
        pattern: IrPattern,
        offset: usize,
        #[serde(rename = "type")]
        value_type: MirType,
    },
    Legacy {
        #[serde(skip_serializing_if = "Option::is_none")]
        dest: Option<u32>,
        source: IrOp,
        offset: usize,
        #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
        value_type: Option<MirType>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MirUnaryKind {
    ToString,
    Not,
    Bang,
    Raise,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MirBinaryKind {
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    CmpIntEq,
    CmpIntNotEq,
    CmpIntLt,
    CmpIntLte,
    CmpIntGt,
    CmpIntGte,
    Concat,
    In,
    PlusPlus,
    MinusMinus,
    Range,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum MirTerminator {
    Return {
        value: u32,
        offset: usize,
    },
    Jump {
        target: u32,
        args: Vec<u32>,
    },
    Match {
        scrutinee: u32,
        arms: Vec<MirMatchArm>,
        offset: usize,
    },
    ShortCircuit {
        op: MirShortCircuitOp,
        condition: u32,
        on_evaluate_rhs: u32,
        on_short_circuit: u32,
        offset: usize,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MirShortCircuitOp {
    AndAnd,
    OrOr,
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MirMatchArm {
    pub(crate) pattern: IrPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) guard_ops: Option<Vec<IrOp>>,
    pub(crate) target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MirLoweringError {
    message: String,
}

impl MirLoweringError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MirLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MirLoweringError {}
