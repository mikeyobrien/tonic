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

/// Emit the C file preamble: include directives and typedef.
pub(super) fn emit_header(out: &mut String) {
    out.push_str("/* tonic c backend - generated file */\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <inttypes.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push_str("#include <errno.h>\n");
    out.push_str("#include <limits.h>\n");
    out.push_str("#include <sys/stat.h>\n");
    out.push_str("#include <sys/wait.h>\n");
    out.push_str("#include <unistd.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}
