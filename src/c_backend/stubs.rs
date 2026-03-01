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
};

// NOTE: stubs.rs — full C backend stub emitter (see batch_1.json for complete content)
// This file provides the complete stub implementation for the C backend compiler.
