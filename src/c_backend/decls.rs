use crate::mir::MirProgram;
use std::collections::BTreeMap;

use super::groups::{group_requires_dispatcher, FunctionGroup};

pub(super) fn emit_forward_declarations(
    groups: &[FunctionGroup],
    mir: &MirProgram,
    clause_symbols: &BTreeMap<usize, String>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) {
    out.push_str("/* forward declarations */\n");
    for group in groups {
        let use_dispatcher = group_requires_dispatcher(group, mir);
        if use_dispatcher {
            for function_index in &group.clause_indices {
                let symbol = clause_symbols
                    .get(function_index)
                    .expect("clause symbol should exist");
                let function = &mir.functions[*function_index];
                let params = (0..function.params.len())
                    .map(|i| format!("TnVal _arg{i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("static TnVal {symbol}({params});\n"));
            }

            let dispatcher_symbol = callable_symbols
                .get(&(group.name.clone(), group.arity))
                .expect("dispatcher symbol should exist");
            let params = (0..group.arity)
                .map(|i| format!("TnVal _arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("static TnVal {dispatcher_symbol}({params});\n"));
        } else {
            let function_index = group.clause_indices[0];
            let symbol = clause_symbols
                .get(&function_index)
                .expect("clause symbol should exist");
            let function = &mir.functions[function_index];
            let params = (0..function.params.len())
                .map(|i| format!("TnVal _arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("static TnVal {symbol}({params});\n"));
        }
    }
    out.push('\n');
}

pub(super) fn emit_main_entrypoint(
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) {
    let entry_symbol = callable_symbols
        .get(&("Demo.run".to_string(), 0))
        .cloned()
        .unwrap_or_else(|| "tn_runtime_error_no_matching_clause".to_string());

    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("  tn_global_argc = argc;\n");
    out.push_str("  tn_global_argv = argv;\n");
    out.push_str(&format!("  TnVal result = {entry_symbol}();\n"));
    out.push_str("  tn_runtime_println(result);\n");
    out.push_str("  tn_runtime_release(result);\n");
    out.push_str("  tn_runtime_gc_collect();\n");
    out.push_str("  tn_runtime_memory_stats_print();\n");
    out.push_str("  return 0;\n");
    out.push_str("}\n");
}
