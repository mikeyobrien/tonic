use super::{mangle_function_name, value_register, LlvmBackendError, LLVM_COMPATIBILITY_VERSION};
use crate::guard_builtins;
use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern};
use crate::mir::{MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator};
use crate::target::TargetTriple;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn lower_mir_subset_to_llvm_ir_impl(
    mir: &MirProgram,
    target: &TargetTriple,
) -> Result<String, LlvmBackendError> {
    let groups = group_functions(mir);
    let mut callable_symbols = BTreeMap::<(String, usize), String>::new();
    let mut clause_symbols = BTreeMap::<usize, String>::new();

    for group in &groups {
        let dispatcher_symbol = mangle_function_name(&group.name, group.arity);
        callable_symbols.insert((group.name.clone(), group.arity), dispatcher_symbol.clone());

        let use_dispatcher = group_requires_dispatcher(group, mir);
        if !use_dispatcher {
            clause_symbols.insert(group.clause_indices[0], dispatcher_symbol);
            continue;
        }

        for (clause_index, function_index) in group.clause_indices.iter().copied().enumerate() {
            clause_symbols.insert(
                function_index,
                format!("{dispatcher_symbol}__clause{clause_index}"),
            );
        }
    }

    let mut lines = vec![
        "; tonic llvm backend mvp".to_string(),
        format!("; llvm_compatibility={LLVM_COMPATIBILITY_VERSION}"),
        format!("target triple = \"{}\"", target.as_str()),
        String::new(),
        "declare i64 @tn_runtime_error_no_matching_clause()".to_string(),
        "declare i64 @tn_runtime_error_bad_match()".to_string(),
        "declare i64 @tn_runtime_error_arity_mismatch()".to_string(),
        "declare i64 @tn_runtime_make_ok(i64)".to_string(),
        "declare i64 @tn_runtime_make_err(i64)".to_string(),
        "declare i64 @tn_runtime_question(i64)".to_string(),
        "declare i64 @tn_runtime_raise(i64)".to_string(),
        "declare i64 @tn_runtime_try(i64)".to_string(),
        "declare i64 @tn_runtime_for(i64)".to_string(),
        "declare i64 @tn_runtime_make_closure(i64, i64, i64)".to_string(),
        "declare i64 (i64, i64, ...) @tn_runtime_call_closure".to_string(),
        "declare i64 @tn_runtime_const_atom(i64)".to_string(),
        "declare i64 @tn_runtime_const_string(i64)".to_string(),
        "declare i64 @tn_runtime_const_float(i64)".to_string(),
        "declare i64 @tn_runtime_to_string(i64)".to_string(),
        "declare i64 @tn_runtime_not(i64)".to_string(),
        "declare i64 @tn_runtime_bang(i64)".to_string(),
        "declare i64 @tn_runtime_load_binding(i64)".to_string(),
        "declare i64 @tn_runtime_match_operator(i64, i64)".to_string(),
        "declare i64 @tn_runtime_make_tuple(i64, i64)".to_string(),
        "declare i64 (i64, ...) @tn_runtime_make_list".to_string(),
        "declare i64 (i64, ...) @tn_runtime_make_bitstring".to_string(),
        "declare i64 @tn_runtime_length(i64)".to_string(),
        "declare i64 @tn_runtime_hd(i64)".to_string(),
        "declare i64 @tn_runtime_tl(i64)".to_string(),
        "declare i64 @tn_runtime_elem(i64, i64)".to_string(),
        "declare i64 @tn_runtime_tuple_size(i64)".to_string(),
        "declare i64 @tn_runtime_put_elem(i64, i64, i64)".to_string(),
        "declare i64 @tn_runtime_map_empty()".to_string(),
        "declare i64 @tn_runtime_make_map(i64, i64)".to_string(),
        "declare i64 @tn_runtime_map_put(i64, i64, i64)".to_string(),
        "declare i64 @tn_runtime_map_update(i64, i64, i64)".to_string(),
        "declare i64 @tn_runtime_map_access(i64, i64)".to_string(),
        "declare i64 @tn_runtime_make_keyword(i64, i64)".to_string(),
        "declare i64 @tn_runtime_keyword_append(i64, i64, i64)".to_string(),
        "declare i64 (i64, ...) @tn_runtime_host_call".to_string(),
        "declare i64 @tn_runtime_protocol_dispatch(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_integer(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_float(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_number(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_atom(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_binary(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_list(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_tuple(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_map(i64)".to_string(),
        "declare i64 @tn_runtime_guard_is_nil(i64)".to_string(),
        "declare i64 @tn_runtime_concat(i64, i64)".to_string(),
        "declare i64 @tn_runtime_in(i64, i64)".to_string(),
        "declare i64 @tn_runtime_list_concat(i64, i64)".to_string(),
        "declare i64 @tn_runtime_list_subtract(i64, i64)".to_string(),
        "declare i64 @tn_runtime_range(i64, i64)".to_string(),
        "declare i1 @tn_runtime_pattern_matches(i64, i64)".to_string(),
        String::new(),
    ];

    for group in &groups {
        let use_dispatcher = group_requires_dispatcher(group, mir);
        if !use_dispatcher {
            let function_index = group.clause_indices[0];
            let function = &mir.functions[function_index];
            let symbol = clause_symbols
                .get(&function_index)
                .expect("clause symbol should exist for single-clause function");
            emit_function(function, symbol, &callable_symbols, &mut lines)?;
            continue;
        }

        for function_index in &group.clause_indices {
            let function = &mir.functions[*function_index];
            let symbol = clause_symbols
                .get(function_index)
                .expect("clause symbol should exist for multi-clause function");
            emit_function(function, symbol, &callable_symbols, &mut lines)?;
        }

        emit_dispatcher(group, mir, &clause_symbols, &callable_symbols, &mut lines)?;
    }

    emit_main_entrypoint(&callable_symbols, &mut lines);

    Ok(lines.join("\n"))
}

#[derive(Debug)]
struct FunctionGroup {
    name: String,
    arity: usize,
    clause_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct PredEdge {
    from: u32,
    args: Vec<u32>,
}

fn group_functions(mir: &MirProgram) -> Vec<FunctionGroup> {
    let mut groups = Vec::<FunctionGroup>::new();
    let mut positions = BTreeMap::<(String, usize), usize>::new();

    for (index, function) in mir.functions.iter().enumerate() {
        let key = (function.name.clone(), function.params.len());
        if let Some(position) = positions.get(&key) {
            groups[*position].clause_indices.push(index);
            continue;
        }

        positions.insert(key, groups.len());
        groups.push(FunctionGroup {
            name: function.name.clone(),
            arity: function.params.len(),
            clause_indices: vec![index],
        });
    }

    groups
}

fn group_requires_dispatcher(group: &FunctionGroup, mir: &MirProgram) -> bool {
    if group.clause_indices.len() > 1 {
        return true;
    }

    let function = &mir.functions[group.clause_indices[0]];
    function.param_patterns.is_some() || function.guard_ops.is_some()
}

fn emit_main_entrypoint(
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) {
    let entry_symbol = callable_symbols
        .get(&("Demo.run".to_string(), 0))
        .cloned()
        .unwrap_or_else(|| "tn_runtime_error_no_matching_clause".to_string());

    lines.push("define i64 @main() {".to_string());
    lines.push("entry:".to_string());
    lines.push(format!("  %main_ret = call i64 @{entry_symbol}()"));
    lines.push("  ret i64 %main_ret".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
}

fn emit_function(
    function: &MirFunction,
    symbol: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(index, _)| format!("i64 %arg{index}"))
        .collect::<Vec<_>>()
        .join(", ");

    let blocks = blocks_by_id(function)?;
    let predecessors = predecessor_edges(function)?;
    let arg_value_ids = infer_block_arg_value_ids(function)?;

    lines.push(format!("define i64 @{symbol}({params}) {{"));

    for block in &function.blocks {
        lines.push(format!("bb{}:", block.id));

        if let Some(arg_ids) = arg_value_ids.get(&block.id) {
            if !arg_ids.is_empty() {
                emit_phi_nodes(function, block, arg_ids, &predecessors, lines)?;
            }
        }

        emit_instructions(function, block, callable_symbols, lines)?;
        emit_terminator(function, block, &blocks, callable_symbols, lines)?;
    }

    lines.push("}".to_string());
    lines.push(String::new());
    Ok(())
}

fn emit_dispatcher(
    group: &FunctionGroup,
    mir: &MirProgram,
    clause_symbols: &BTreeMap<usize, String>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    let dispatcher_symbol = mangle_function_name(&group.name, group.arity);
    let params = (0..group.arity)
        .map(|index| format!("i64 %arg{index}"))
        .collect::<Vec<_>>()
        .join(", ");

    lines.push(format!("define i64 @{dispatcher_symbol}({params}) {{"));
    lines.push("entry:".to_string());

    for (clause_index, function_index) in group.clause_indices.iter().copied().enumerate() {
        let function = &mir.functions[function_index];
        let clause_symbol = clause_symbols
            .get(&function_index)
            .expect("clause symbol should exist for dispatcher clause");
        let call_label = format!("dispatcher_clause_{clause_index}_call");
        let next_label = format!("dispatcher_clause_{clause_index}_next");

        let mut condition_terms = Vec::<String>::new();
        if let Some(patterns) = &function.param_patterns {
            for (param_index, pattern) in patterns.iter().enumerate() {
                let condition = emit_pattern_condition(
                    &function.name,
                    &format!("%arg{param_index}"),
                    pattern,
                    &format!("dispatcher_clause_{clause_index}_pattern_{param_index}"),
                    lines,
                )?;
                condition_terms.push(condition);
            }
        }

        if let Some(guard_ops) = &function.guard_ops {
            let guard_condition = emit_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &format!("dispatcher_clause_{clause_index}_guard"),
                callable_symbols,
                lines,
            )?;
            condition_terms.push(guard_condition);
        }

        let condition = combine_conditions(
            &function.name,
            condition_terms,
            &format!("dispatcher_clause_{clause_index}_condition"),
            lines,
        )?;

        if clause_index + 1 == group.clause_indices.len() {
            lines.push(format!(
                "  br i1 {condition}, label %{call_label}, label %dispatcher_no_matching_clause"
            ));
        } else {
            lines.push(format!(
                "  br i1 {condition}, label %{call_label}, label %{next_label}"
            ));
        }

        lines.push(format!("{call_label}:"));
        let call_args = (0..group.arity)
            .map(|index| format!("i64 %arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "  %dispatcher_ret_{clause_index} = call i64 @{clause_symbol}({call_args})"
        ));
        lines.push(format!("  ret i64 %dispatcher_ret_{clause_index}"));

        if clause_index + 1 != group.clause_indices.len() {
            lines.push(format!("{next_label}:"));
        }
    }

    lines.push("dispatcher_no_matching_clause:".to_string());
    lines.push(
        "  %dispatcher_no_clause = call i64 @tn_runtime_error_no_matching_clause()".to_string(),
    );
    lines.push("  ret i64 %dispatcher_no_clause".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    Ok(())
}

fn blocks_by_id(function: &MirFunction) -> Result<BTreeMap<u32, &MirBlock>, LlvmBackendError> {
    let mut blocks = BTreeMap::new();

    for block in &function.blocks {
        if blocks.insert(block.id, block).is_some() {
            return Err(LlvmBackendError::new(format!(
                "llvm backend duplicate block {} in function {}",
                block.id, function.name
            )));
        }
    }

    if !blocks.contains_key(&function.entry_block) {
        return Err(LlvmBackendError::new(format!(
            "llvm backend missing entry block {} in function {}",
            function.entry_block, function.name
        )));
    }

    Ok(blocks)
}

#[path = "codegen_analysis.rs"]
mod analysis;
use analysis::*;

#[path = "codegen_inst.rs"]
mod inst;
use inst::*;

#[path = "codegen_builtins.rs"]
mod builtins;
use builtins::*;

#[path = "codegen_term.rs"]
mod term;
use term::*;

#[path = "codegen_patterns.rs"]
mod patterns;
use patterns::*;
