mod codegen;
#[cfg(test)]
mod tests;

use crate::ir::IrOp;
use crate::mir::{MirInstruction, MirProgram};
use std::fmt;

pub(crate) const LLVM_COMPATIBILITY_VERSION: &str = "18.1.8";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlvmBackendError {
    message: String,
}

impl LlvmBackendError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn unsupported_instruction(
        function: &str,
        instruction: &MirInstruction,
        offset: usize,
    ) -> Self {
        let op = instruction_name(instruction);
        Self::new(format!(
            "llvm backend unsupported instruction {op} in function {function} at offset {offset}"
        ))
    }

    pub(crate) fn unsupported_guard_op(function: &str, op: &IrOp, offset: usize) -> Self {
        let op_name = ir_op_name(op);
        Self::new(format!(
            "llvm backend unsupported guard op {op_name} in function {function} at offset {offset}"
        ))
    }
}

impl fmt::Display for LlvmBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LlvmBackendError {}

pub(crate) fn lower_mir_subset_to_llvm_ir(mir: &MirProgram) -> Result<String, LlvmBackendError> {
    codegen::lower_mir_subset_to_llvm_ir_impl(mir)
}

pub(crate) fn mangle_function_name(name: &str, arity: usize) -> String {
    format!("tn_{}__arity{arity}", sanitize_identifier(name))
}

fn sanitize_identifier(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

pub(crate) fn value_register(id: u32) -> String {
    format!("%v{id}")
}

pub(crate) fn instruction_name(instruction: &MirInstruction) -> &'static str {
    match instruction {
        MirInstruction::ConstInt { .. } => "const_int",
        MirInstruction::ConstFloat { .. } => "const_float",
        MirInstruction::ConstBool { .. } => "const_bool",
        MirInstruction::ConstNil { .. } => "const_nil",
        MirInstruction::ConstString { .. } => "const_string",
        MirInstruction::ConstAtom { .. } => "const_atom",
        MirInstruction::LoadVariable { .. } => "load_variable",
        MirInstruction::Unary { .. } => "unary",
        MirInstruction::Binary { .. } => "binary",
        MirInstruction::Call { .. } => "call",
        MirInstruction::CallValue { .. } => "call_value",
        MirInstruction::MakeClosure { .. } => "make_closure",
        MirInstruction::Question { .. } => "question",
        MirInstruction::MatchPattern { .. } => "match_pattern",
        MirInstruction::Legacy { .. } => "legacy",
    }
}

fn ir_op_name(op: &IrOp) -> &'static str {
    match op {
        IrOp::ConstInt { .. } => "const_int",
        IrOp::ConstFloat { .. } => "const_float",
        IrOp::ConstBool { .. } => "const_bool",
        IrOp::ConstNil { .. } => "const_nil",
        IrOp::ConstString { .. } => "const_string",
        IrOp::ToString { .. } => "to_string",
        IrOp::Call { .. } => "call",
        IrOp::MakeClosure { .. } => "make_closure",
        IrOp::CallValue { .. } => "call_value",
        IrOp::Question { .. } => "question",
        IrOp::Case { .. } => "case",
        IrOp::Try { .. } => "try",
        IrOp::Raise { .. } => "raise",
        IrOp::For { .. } => "for",
        IrOp::LoadVariable { .. } => "load_variable",
        IrOp::ConstAtom { .. } => "const_atom",
        IrOp::AddInt { .. } => "add_int",
        IrOp::SubInt { .. } => "sub_int",
        IrOp::MulInt { .. } => "mul_int",
        IrOp::DivInt { .. } => "div_int",
        IrOp::CmpInt { .. } => "cmp_int",
        IrOp::Not { .. } => "not",
        IrOp::Bang { .. } => "bang",
        IrOp::AndAnd { .. } => "and_and",
        IrOp::OrOr { .. } => "or_or",
        IrOp::And { .. } => "and",
        IrOp::Or { .. } => "or",
        IrOp::Concat { .. } => "concat",
        IrOp::In { .. } => "in",
        IrOp::PlusPlus { .. } => "plus_plus",
        IrOp::MinusMinus { .. } => "minus_minus",
        IrOp::Range { .. } => "range",
        IrOp::Match { .. } => "match",
        IrOp::Return { .. } => "return",
    }
}
