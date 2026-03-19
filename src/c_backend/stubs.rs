use crate::mir::MirProgram;

use super::{
    error::CBackendError, runtime_patterns::emit_runtime_pattern_helpers,
    stubs_closures::emit_compiled_closure_helpers, stubs_constructors::emit_stubs_constructors,
    stubs_for::emit_runtime_for_helpers, stubs_host_dispatch::emit_stubs_host_dispatch,
    stubs_host_http::emit_stubs_host_http, stubs_host_path::emit_stubs_host_path,
    stubs_host_sys::emit_stubs_host_sys, stubs_io::emit_stubs_io, stubs_map::emit_stubs_map,
    stubs_memory::emit_stubs_memory, stubs_results::emit_stubs_results,
    stubs_try::emit_runtime_try_helpers, stubs_types::emit_stubs_types,
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
    out.push_str("#include <dirent.h>\n");
    out.push_str("#include <sys/stat.h>\n");
    out.push_str("#include <sys/wait.h>\n");
    out.push_str("#include <unistd.h>\n");
    out.push_str("#include <sys/socket.h>\n");
    out.push_str("#include <netinet/in.h>\n");
    out.push_str("#include <arpa/inet.h>\n");
    out.push_str("#include <sys/select.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}

/// Emit runtime helper definitions for the generated C program.
///
/// Task 05 helpers are implemented inline; unsupported helpers remain explicit
/// abort stubs so failures stay deterministic.
pub(super) fn emit_runtime_stubs(mir: &MirProgram, out: &mut String) -> Result<(), CBackendError> {
    emit_stubs_types(out);
    emit_stubs_memory(out);
    emit_stubs_constructors(out);
    emit_stubs_map(out);
    emit_stubs_io(out);
    emit_stubs_host_dispatch(out);
    emit_stubs_host_path(out);
    emit_stubs_host_sys(out);
    emit_stubs_host_http(out);
    emit_stubs_results(out);
    emit_runtime_pattern_helpers(mir, out)?;
    emit_runtime_try_helpers(mir, out)?;
    emit_runtime_for_helpers(mir, out)?;
    emit_compiled_closure_helpers(mir, out)?;
    Ok(())
}

pub(super) fn pop_stack_value(
    stack: &mut Vec<String>,
    context: &str,
) -> Result<String, CBackendError> {
    stack.pop().ok_or_else(|| {
        CBackendError::new(format!("c backend closure stack underflow for {context}"))
    })
}

pub(super) fn c_string_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
