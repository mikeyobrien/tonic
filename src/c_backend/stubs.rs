/// Emit the C file preamble: include directives and typedef.
pub(super) fn emit_header(out: &mut String) {
    out.push_str("/* tonic c backend - generated file */\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <inttypes.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}

/// Emit weak stub definitions for every `tn_runtime_*` function.
///
/// Stubs abort with a descriptive message so programs that only use integer
/// arithmetic compile and run correctly, while programs that depend on
/// unimplemented runtime operations fail at runtime with a clear diagnostic
/// rather than a linker error.
pub(super) fn emit_runtime_stubs(out: &mut String) {
    out.push_str("/* runtime stubs - operations not yet natively implemented */\n");
    out.push_str("static TnVal tn_stub_abort(const char *name) {\n");
    out.push_str("  fprintf(stderr, \"error: native runtime not available for '%s'\\n\", name);\n");
    out.push_str("  exit(1);\n");
    out.push_str("}\n\n");

    // Unary / error stubs
    for name in &[
        "tn_runtime_error_no_matching_clause",
        "tn_runtime_error_bad_match",
        "tn_runtime_error_arity_mismatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(void) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Single-arg stubs
    for name in &[
        "tn_runtime_make_ok",
        "tn_runtime_make_err",
        "tn_runtime_question",
        "tn_runtime_raise",
        "tn_runtime_try",
        "tn_runtime_const_atom",
        "tn_runtime_to_string",
        "tn_runtime_not",
        "tn_runtime_bang",
        "tn_runtime_load_binding",
        "tn_runtime_protocol_dispatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Two-arg stubs
    for name in &[
        "tn_runtime_match_operator",
        "tn_runtime_make_tuple",
        "tn_runtime_make_map",
        "tn_runtime_make_keyword",
        "tn_runtime_map_access",
        "tn_runtime_concat",
        "tn_runtime_in",
        "tn_runtime_list_concat",
        "tn_runtime_list_subtract",
        "tn_runtime_range",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a, TnVal _b) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Three-arg stubs
    for name in &[
        "tn_runtime_make_closure",
        "tn_runtime_map_put",
        "tn_runtime_map_update",
        "tn_runtime_keyword_append",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a, TnVal _b, TnVal _c) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Zero-arg and special stubs
    out.push_str("static TnVal tn_runtime_map_empty(void) { return tn_stub_abort(\"tn_runtime_map_empty\"); }\n");
    out.push_str("static int tn_runtime_pattern_matches(TnVal _v, TnVal _p) { (void)tn_stub_abort(\"tn_runtime_pattern_matches\"); return 0; }\n");
    out.push('\n');

    // Variadic stubs: list construction, host calls, closure calls.
    // These use an explicit leading count argument followed by the elements.
    out.push_str("static TnVal tn_runtime_make_list_varargs(TnVal _count, ...) {\n");
    out.push_str("  return tn_stub_abort(\"tn_runtime_make_list\");\n");
    out.push_str("}\n");
    out.push_str("static TnVal tn_runtime_host_call_varargs(TnVal _count, ...) {\n");
    out.push_str("  return tn_stub_abort(\"tn_runtime_host_call\");\n");
    out.push_str("}\n");
    out.push_str(
        "static TnVal tn_runtime_call_closure_varargs(TnVal _closure, TnVal _count, ...) {\n",
    );
    out.push_str("  return tn_stub_abort(\"tn_runtime_call_closure\");\n");
    out.push_str("}\n");
    out.push('\n');
}
