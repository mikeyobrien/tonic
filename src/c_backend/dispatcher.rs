use crate::llvm_backend::mangle_function_name;
use crate::mir::MirProgram;
use std::collections::BTreeMap;

use super::error::CBackendError;
use super::groups::FunctionGroup;
use super::terminator::{emit_c_guard_condition, emit_c_pattern_condition};

pub(super) fn emit_dispatcher(
    group: &FunctionGroup,
    mir: &MirProgram,
    clause_symbols: &BTreeMap<usize, String>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let dispatcher_symbol = mangle_function_name(&group.name, group.arity);
    let params = (0..group.arity)
        .map(|i| format!("TnVal _arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = (0..group.arity)
        .map(|i| format!("_arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!("static TnVal {dispatcher_symbol}({params}) {{\n"));

    for (clause_index, function_index) in group.clause_indices.iter().copied().enumerate() {
        let function = &mir.functions[function_index];
        let clause_symbol = clause_symbols
            .get(&function_index)
            .expect("clause symbol should exist");

        let mut condition_terms: Vec<String> = Vec::new();

        if let Some(patterns) = &function.param_patterns {
            for (param_index, pattern) in patterns.iter().enumerate() {
                let label = format!("disp{clause_index}_pat{param_index}");
                let cond = emit_c_pattern_condition(
                    &function.name,
                    &format!("_arg{param_index}"),
                    pattern,
                    &label,
                    out,
                )?;
                condition_terms.push(cond);
            }
        }

        if let Some(guard_ops) = &function.guard_ops {
            let guard_label = format!("disp{clause_index}_guard");
            let guard_cond = emit_c_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &guard_label,
                callable_symbols,
                out,
            )?;
            condition_terms.push(guard_cond);
        }

        let full_cond = if condition_terms.is_empty() {
            "1".to_string()
        } else {
            condition_terms
                .iter()
                .map(|c| format!("({c})"))
                .collect::<Vec<_>>()
                .join(" && ")
        };

        if clause_index + 1 == group.clause_indices.len() {
            out.push_str(&format!(
                "  if ({full_cond}) {{ return {clause_symbol}({call_args}); }}\n"
            ));
            out.push_str("  return tn_runtime_error_no_matching_clause();\n");
        } else {
            out.push_str(&format!(
                "  if ({full_cond}) {{ return {clause_symbol}({call_args}); }}\n"
            ));
        }
    }

    out.push_str("}\n\n");
    Ok(())
}
