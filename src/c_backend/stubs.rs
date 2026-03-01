use crate::ir::{CmpKind, IrCallTarget, IrForGenerator, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;
use crate::mir::{MirInstruction, MirProgram};
use std::collections::BTreeMap;

use super::{
    error::CBackendError,
    hash::{
        closure_capture_names, hash_closure_descriptor_i64, hash_ir_op_i64, hash_pattern_i64,
        hash_text_i64,
    },
    runtime_patterns::emit_runtime_pattern_helpers,
};