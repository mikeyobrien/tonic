use crate::llvm_backend::instruction_name;
use crate::mir::MirInstruction;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CBackendError {
    message: String,
}

impl CBackendError {
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
            "c backend unsupported instruction {op} in function {function} at offset {offset}"
        ))
    }
}

impl fmt::Display for CBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CBackendError {}
