use super::{mangle_function_name, value_register, LlvmBackendError, LLVM_COMPATIBILITY_VERSION};
use crate::guard_builtins;
use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern};
use crate::mir::{MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn lower_mir_subset_to_llvm_ir_impl(
    mir: &MirProgram,
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
        "target triple = \"x86_64-unknown-linux-gnu\"".to_string(),
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

fn predecessor_edges(
    function: &MirFunction,
) -> Result<BTreeMap<u32, Vec<PredEdge>>, LlvmBackendError> {
    let mut predecessors = BTreeMap::<u32, Vec<PredEdge>>::new();
    let known_blocks = function
        .blocks
        .iter()
        .map(|block| block.id)
        .collect::<BTreeSet<_>>();

    for block in &function.blocks {
        match &block.terminator {
            MirTerminator::Jump { target, args } => {
                if !known_blocks.contains(target) {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend missing jump target block {} in function {}",
                        target, function.name
                    )));
                }

                predecessors.entry(*target).or_default().push(PredEdge {
                    from: block.id,
                    args: args.clone(),
                });
            }
            MirTerminator::Match { arms, .. } => {
                for arm in arms {
                    if !known_blocks.contains(&arm.target) {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend missing match target block {} in function {}",
                            arm.target, function.name
                        )));
                    }
                    predecessors.entry(arm.target).or_default().push(PredEdge {
                        from: block.id,
                        args: Vec::new(),
                    });
                }
            }
            MirTerminator::ShortCircuit {
                on_evaluate_rhs,
                on_short_circuit,
                ..
            } => {
                for target in [on_evaluate_rhs, on_short_circuit] {
                    if !known_blocks.contains(target) {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend missing short-circuit target block {} in function {}",
                            target, function.name
                        )));
                    }
                    predecessors.entry(*target).or_default().push(PredEdge {
                        from: block.id,
                        args: Vec::new(),
                    });
                }
            }
            MirTerminator::Return { .. } => {}
        }
    }

    for edges in predecessors.values_mut() {
        edges.sort_by_key(|edge| edge.from);
    }

    Ok(predecessors)
}

fn infer_block_arg_value_ids(
    function: &MirFunction,
) -> Result<BTreeMap<u32, Vec<u32>>, LlvmBackendError> {
    let unresolved_value_ids = unresolved_value_ids(function);
    let mut assigned_value_ids = BTreeSet::<u32>::new();
    let mut value_ids_by_block = BTreeMap::new();

    for block in &function.blocks {
        if block.args.is_empty() {
            value_ids_by_block.insert(block.id, Vec::new());
            continue;
        }

        let mut inferred = Vec::<u32>::new();

        for candidate in block_external_value_ids(block) {
            if !unresolved_value_ids.contains(&candidate)
                || assigned_value_ids.contains(&candidate)
                || inferred.contains(&candidate)
            {
                continue;
            }
            inferred.push(candidate);
            if inferred.len() == block.args.len() {
                break;
            }
        }

        if inferred.len() < block.args.len() {
            for candidate in &unresolved_value_ids {
                if assigned_value_ids.contains(candidate) || inferred.contains(candidate) {
                    continue;
                }
                inferred.push(*candidate);
                if inferred.len() == block.args.len() {
                    break;
                }
            }
        }

        if inferred.len() != block.args.len() {
            return Err(LlvmBackendError::new(format!(
                "llvm backend cannot infer block arg values for block {} in function {}",
                block.id, function.name
            )));
        }

        assigned_value_ids.extend(inferred.iter().copied());
        value_ids_by_block.insert(block.id, inferred);
    }

    Ok(value_ids_by_block)
}

fn block_external_value_ids(block: &MirBlock) -> Vec<u32> {
    let mut defined = BTreeSet::<u32>::new();
    let mut ordered_external = Vec::<u32>::new();

    for instruction in &block.instructions {
        for used in instruction_operands(instruction) {
            if !defined.contains(&used) && !ordered_external.contains(&used) {
                ordered_external.push(used);
            }
        }

        if let Some(dest) = instruction_destination(instruction) {
            defined.insert(dest);
        }
    }

    for used in terminator_operands(&block.terminator) {
        if !defined.contains(&used) && !ordered_external.contains(&used) {
            ordered_external.push(used);
        }
    }

    ordered_external
}

fn unresolved_value_ids(function: &MirFunction) -> BTreeSet<u32> {
    let mut defined = BTreeSet::<u32>::new();
    let mut referenced = BTreeSet::<u32>::new();

    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Some(dest) = instruction_destination(instruction) {
                defined.insert(dest);
            }
            for used in instruction_operands(instruction) {
                referenced.insert(used);
            }
        }

        for used in terminator_operands(&block.terminator) {
            referenced.insert(used);
        }
    }

    referenced
        .into_iter()
        .filter(|value_id| !defined.contains(value_id))
        .collect()
}

fn emit_phi_nodes(
    function: &MirFunction,
    block: &MirBlock,
    arg_ids: &[u32],
    predecessors: &BTreeMap<u32, Vec<PredEdge>>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    let Some(incoming_edges) = predecessors.get(&block.id) else {
        return Err(LlvmBackendError::new(format!(
            "llvm backend missing predecessors for block {} in function {}",
            block.id, function.name
        )));
    };

    for (arg_index, dest) in arg_ids.iter().copied().enumerate() {
        let mut incoming = Vec::with_capacity(incoming_edges.len());

        for edge in incoming_edges {
            if edge.args.len() != block.args.len() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend jump argument mismatch into block {} in function {}",
                    block.id, function.name
                )));
            }

            incoming.push(format!(
                "[ {}, %bb{} ]",
                value_register(edge.args[arg_index]),
                edge.from
            ));
        }

        lines.push(format!(
            "  {} = phi i64 {}",
            value_register(dest),
            incoming.join(", ")
        ));
    }

    Ok(())
}

fn emit_instructions(
    function: &MirFunction,
    block: &MirBlock,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    for instruction in &block.instructions {
        match instruction {
            MirInstruction::ConstInt { dest, value, .. } => {
                lines.push(format!("  {} = add i64 0, {value}", value_register(*dest)));
            }
            MirInstruction::ConstBool { dest, value, .. } => {
                lines.push(format!(
                    "  {} = add i64 0, {}",
                    value_register(*dest),
                    i64::from(*value)
                ));
            }
            MirInstruction::ConstNil { dest, .. } => {
                lines.push(format!("  {} = add i64 0, 0", value_register(*dest)));
            }
            MirInstruction::ConstAtom { dest, value, .. } => {
                let atom_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_atom(i64 {atom_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::ConstString { dest, value, .. } => {
                let string_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_string(i64 {string_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::ConstFloat { dest, value, .. } => {
                let float_hash = hash_text_i64(value);
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_const_float(i64 {float_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::LoadVariable { dest, name, .. } => {
                if let Some(param_index) =
                    function.params.iter().position(|param| param.name == *name)
                {
                    lines.push(format!(
                        "  {} = add i64 0, %arg{param_index}",
                        value_register(*dest)
                    ));
                } else {
                    let binding_hash = hash_text_i64(name);
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_load_binding(i64 {binding_hash})",
                        value_register(*dest)
                    ));
                }
            }
            MirInstruction::Unary {
                dest, kind, input, ..
            } => match kind {
                crate::mir::MirUnaryKind::Raise => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_raise(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::ToString => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_to_string(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::Not => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_not(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::Bang => {
                    lines.push(format!(
                        "  {} = call i64 @tn_runtime_bang(i64 {})",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
                crate::mir::MirUnaryKind::BitwiseNot => {
                    lines.push(format!(
                        "  {} = xor i64 {}, -1",
                        value_register(*dest),
                        value_register(*input)
                    ));
                }
            },
            MirInstruction::Question { dest, input, .. } => {
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_question(i64 {})",
                    value_register(*dest),
                    value_register(*input)
                ));
            }
            MirInstruction::Legacy {
                dest,
                source,
                offset,
                ..
            } => {
                let runtime_helper = match source {
                    IrOp::Try { .. } => "tn_runtime_try",
                    IrOp::For { .. } => "tn_runtime_for",
                    _ => {
                        return Err(LlvmBackendError::unsupported_instruction(
                            &function.name,
                            instruction,
                            *offset,
                        ));
                    }
                };

                let op_hash = hash_ir_op_i64(source)?;
                let Some(dest) = dest else {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend missing legacy destination in function {} at offset {}",
                        function.name, offset
                    )));
                };

                lines.push(format!(
                    "  {} = call i64 @{runtime_helper}(i64 {op_hash})",
                    value_register(*dest)
                ));
            }
            MirInstruction::MakeClosure {
                dest, params, ops, ..
            } => {
                let capture_names = closure_capture_names(params, ops);
                let descriptor_hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_make_closure(i64 {descriptor_hash}, i64 {}, i64 {})",
                    value_register(*dest),
                    params.len(),
                    capture_names.len()
                ));
            }
            MirInstruction::CallValue {
                dest, callee, args, ..
            } => {
                let mut rendered_args = vec![
                    format!("i64 {}", value_register(*callee)),
                    format!("i64 {}", args.len()),
                ];
                rendered_args.extend(
                    args.iter()
                        .map(|arg| format!("i64 {}", value_register(*arg))),
                );
                lines.push(format!(
                    "  {} = call i64 (i64, i64, ...) @tn_runtime_call_closure({})",
                    value_register(*dest),
                    rendered_args.join(", ")
                ));
            }
            MirInstruction::Binary {
                dest,
                kind,
                left,
                right,
                offset: _,
                ..
            } => match kind {
                MirBinaryKind::AddInt
                | MirBinaryKind::SubInt
                | MirBinaryKind::MulInt
                | MirBinaryKind::DivInt => {
                    let op = match kind {
                        MirBinaryKind::AddInt => "add",
                        MirBinaryKind::SubInt => "sub",
                        MirBinaryKind::MulInt => "mul",
                        MirBinaryKind::DivInt => "sdiv",
                        _ => unreachable!(),
                    };

                    lines.push(format!(
                        "  {} = {op} i64 {}, {}",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
                MirBinaryKind::CmpIntEq
                | MirBinaryKind::CmpIntNotEq
                | MirBinaryKind::CmpIntLt
                | MirBinaryKind::CmpIntLte
                | MirBinaryKind::CmpIntGt
                | MirBinaryKind::CmpIntGte => {
                    let predicate = match kind {
                        MirBinaryKind::CmpIntEq => "eq",
                        MirBinaryKind::CmpIntNotEq => "ne",
                        MirBinaryKind::CmpIntLt => "slt",
                        MirBinaryKind::CmpIntLte => "sle",
                        MirBinaryKind::CmpIntGt => "sgt",
                        MirBinaryKind::CmpIntGte => "sge",
                        _ => unreachable!(),
                    };

                    lines.push(format!(
                        "  %cmp_{dest} = icmp {predicate} i64 {}, {}",
                        value_register(*left),
                        value_register(*right)
                    ));
                    lines.push(format!(
                        "  {} = zext i1 %cmp_{dest} to i64",
                        value_register(*dest)
                    ));
                }
                MirBinaryKind::Concat
                | MirBinaryKind::In
                | MirBinaryKind::PlusPlus
                | MirBinaryKind::MinusMinus
                | MirBinaryKind::Range
                | MirBinaryKind::NotIn
                | MirBinaryKind::SteppedRange => {
                    let helper = match kind {
                        MirBinaryKind::Concat => "tn_runtime_concat",
                        MirBinaryKind::In => "tn_runtime_in",
                        MirBinaryKind::PlusPlus => "tn_runtime_list_concat",
                        MirBinaryKind::MinusMinus => "tn_runtime_list_subtract",
                        MirBinaryKind::Range => "tn_runtime_range",
                        MirBinaryKind::NotIn => "tn_runtime_not_in",
                        MirBinaryKind::SteppedRange => "tn_runtime_stepped_range",
                        _ => unreachable!(),
                    };
                    lines.push(format!(
                        "  {} = call i64 @{helper}(i64 {}, i64 {})",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
                MirBinaryKind::BitwiseAnd
                | MirBinaryKind::BitwiseOr
                | MirBinaryKind::BitwiseXor
                | MirBinaryKind::BitwiseShiftLeft
                | MirBinaryKind::BitwiseShiftRight => {
                    let op = match kind {
                        MirBinaryKind::BitwiseAnd => "and",
                        MirBinaryKind::BitwiseOr => "or",
                        MirBinaryKind::BitwiseXor => "xor",
                        MirBinaryKind::BitwiseShiftLeft => "shl",
                        MirBinaryKind::BitwiseShiftRight => "ashr",
                        _ => unreachable!(),
                    };
                    lines.push(format!(
                        "  {} = {op} i64 {}, {}",
                        value_register(*dest),
                        value_register(*left),
                        value_register(*right)
                    ));
                }
            },
            MirInstruction::Call {
                dest,
                callee,
                args,
                offset,
                ..
            } => match callee {
                IrCallTarget::Builtin { name } => {
                    emit_builtin_call_from_value_ids(
                        *dest,
                        name,
                        args,
                        &function.name,
                        *offset,
                        lines,
                    )?;
                }
                IrCallTarget::Function { name } => {
                    let key = (name.clone(), args.len());
                    if let Some(symbol) = callable_symbols.get(&key) {
                        let rendered_args = args
                            .iter()
                            .map(|id| format!("i64 {}", value_register(*id)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        lines.push(format!(
                            "  {} = call i64 @{symbol}({rendered_args})",
                            value_register(*dest)
                        ));
                        continue;
                    }

                    if callable_symbols
                        .keys()
                        .any(|(candidate, _)| candidate == name)
                    {
                        lines.push(format!(
                            "  {} = call i64 @tn_runtime_error_arity_mismatch()",
                            value_register(*dest)
                        ));
                        continue;
                    }

                    return Err(LlvmBackendError::new(format!(
                        "llvm backend unknown function call target {name} in function {} at offset {offset}",
                        function.name
                    )));
                }
            },
            MirInstruction::MatchPattern {
                dest,
                input,
                pattern,
                ..
            } => {
                let pattern_hash = hash_pattern_i64(pattern)?;
                lines.push(format!(
                    "  {} = call i64 @tn_runtime_match_operator(i64 {}, i64 {pattern_hash})",
                    value_register(*dest),
                    value_register(*input),
                ));
            }
        }
    }

    Ok(())
}

fn emit_builtin_call_from_value_ids(
    dest: u32,
    builtin: &str,
    args: &[u32],
    function_name: &str,
    offset: usize,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("i64 {}", value_register(*id)))
        .collect::<Vec<_>>();

    emit_builtin_call_from_registers(
        value_register(dest),
        builtin,
        rendered_args,
        function_name,
        offset,
        lines,
    )
}

fn emit_builtin_call_from_registers(
    dest: String,
    builtin: &str,
    rendered_args: Vec<String>,
    function_name: &str,
    offset: usize,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    if let Some(helper) = guard_builtins::llvm_helper_name(builtin) {
        if rendered_args.len() != guard_builtins::GUARD_BUILTIN_ARITY {
            return Err(LlvmBackendError::new(format!(
                "llvm backend builtin {builtin} arity mismatch in function {function_name} at offset {offset}"
            )));
        }

        lines.push(format!(
            "  {dest} = call i64 @{helper}({})",
            rendered_args[0]
        ));
        return Ok(());
    }

    match builtin {
        "ok" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin ok arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_ok({})",
                rendered_args[0]
            ));
        }
        "err" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin err arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_err({})",
                rendered_args[0]
            ));
        }
        "tuple" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin tuple arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_tuple({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "list" => {
            let mut call_args = vec![format!("i64 {}", rendered_args.len())];
            call_args.extend(rendered_args);
            lines.push(format!(
                "  {dest} = call i64 (i64, ...) @tn_runtime_make_list({})",
                call_args.join(", ")
            ));
        }
        "map_empty" => {
            if !rendered_args.is_empty() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_empty arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!("  {dest} = call i64 @tn_runtime_map_empty()"));
        }
        "map" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_map({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "map_put" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_put arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_put({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "map_update" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_update arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_update({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "map_access" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin map_access arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_map_access({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "keyword" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin keyword arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_make_keyword({}, {})",
                rendered_args[0], rendered_args[1]
            ));
        }
        "keyword_append" => {
            if rendered_args.len() != 3 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin keyword_append arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_keyword_append({}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2]
            ));
        }
        "host_call" => {
            if rendered_args.is_empty() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin host_call arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            let mut call_args = vec![format!("i64 {}", rendered_args.len())];
            call_args.extend(rendered_args);
            lines.push(format!(
                "  {dest} = call i64 (i64, ...) @tn_runtime_host_call({})",
                call_args.join(", ")
            ));
        }
        "protocol_dispatch" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin protocol_dispatch arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_protocol_dispatch({})",
                rendered_args[0]
            ));
        }
        "div" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin div arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            // Integer division truncating toward zero (sdiv)
            lines.push(format!(
                "  {dest} = sdiv i64 {}, {}",
                rendered_args[0], rendered_args[1]
            ));
        }
        "rem" => {
            if rendered_args.len() != 2 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin rem arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            // Integer remainder (srem)
            lines.push(format!(
                "  {dest} = srem i64 {}, {}",
                rendered_args[0], rendered_args[1]
            ));
        }
        "byte_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin byte_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_byte_size(i64 {})",
                rendered_args[0]
            ));
        }
        "bit_size" => {
            if rendered_args.len() != 1 {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend builtin bit_size arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            lines.push(format!(
                "  {dest} = call i64 @tn_runtime_bit_size(i64 {})",
                rendered_args[0]
            ));
        }
        other => {
            return Err(LlvmBackendError::new(format!(
                "llvm backend unsupported builtin call target {other} in function {function_name} at offset {offset}"
            )));
        }
    }

    Ok(())
}

fn emit_terminator(
    function: &MirFunction,
    block: &MirBlock,
    blocks: &BTreeMap<u32, &MirBlock>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    match &block.terminator {
        MirTerminator::Return { value, .. } => {
            lines.push(format!("  ret i64 {}", value_register(*value)));
            Ok(())
        }
        MirTerminator::Jump { target, args } => {
            let Some(target_block) = blocks.get(target) else {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend missing jump target block {} in function {}",
                    target, function.name
                )));
            };

            if args.len() != target_block.args.len() {
                return Err(LlvmBackendError::new(format!(
                    "llvm backend jump argument mismatch into block {} in function {}",
                    target, function.name
                )));
            }

            lines.push(format!("  br label %bb{target}"));
            Ok(())
        }
        MirTerminator::ShortCircuit {
            op,
            condition,
            on_evaluate_rhs,
            on_short_circuit,
            ..
        } => {
            let condition_bool = format!("%sc_cond_{}", block.id);
            lines.push(format!(
                "  {condition_bool} = icmp ne i64 {}, 0",
                value_register(*condition)
            ));

            let (true_target, false_target) = match op {
                crate::mir::MirShortCircuitOp::AndAnd | crate::mir::MirShortCircuitOp::And => {
                    (on_evaluate_rhs, on_short_circuit)
                }
                crate::mir::MirShortCircuitOp::OrOr | crate::mir::MirShortCircuitOp::Or => {
                    (on_short_circuit, on_evaluate_rhs)
                }
            };

            lines.push(format!(
                "  br i1 {condition_bool}, label %bb{true_target}, label %bb{false_target}"
            ));
            Ok(())
        }
        MirTerminator::Match {
            scrutinee,
            arms,
            offset,
        } => emit_match_terminator(
            function,
            block,
            *scrutinee,
            arms,
            *offset,
            callable_symbols,
            lines,
        ),
    }
}

fn emit_match_terminator(
    function: &MirFunction,
    block: &MirBlock,
    scrutinee: u32,
    arms: &[crate::mir::MirMatchArm],
    _offset: usize,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<(), LlvmBackendError> {
    if arms.is_empty() {
        lines.push(
            "  %match_no_clause = call i64 @tn_runtime_error_no_matching_clause()".to_string(),
        );
        lines.push("  ret i64 %match_no_clause".to_string());
        return Ok(());
    }

    let scrutinee_operand = value_register(scrutinee);

    for (arm_index, arm) in arms.iter().enumerate() {
        let pattern_condition = emit_pattern_condition(
            &function.name,
            &scrutinee_operand,
            &arm.pattern,
            &format!("match_block{}_arm{arm_index}_pattern", block.id),
            lines,
        )?;

        let mut condition_terms = vec![pattern_condition];
        if let Some(guard_ops) = &arm.guard_ops {
            let guard_condition = emit_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &format!("match_block{}_arm{arm_index}_guard", block.id),
                callable_symbols,
                lines,
            )?;
            condition_terms.push(guard_condition);
        }

        let condition = combine_conditions(
            &function.name,
            condition_terms,
            &format!("match_block{}_arm{arm_index}_condition", block.id),
            lines,
        )?;

        if arm_index + 1 == arms.len() {
            lines.push(format!(
                "  br i1 {condition}, label %bb{}, label %match_block{}_no_clause",
                arm.target, block.id
            ));
            lines.push(format!("match_block{}_no_clause:", block.id));
            lines.push(
                "  %match_no_clause = call i64 @tn_runtime_error_no_matching_clause()".to_string(),
            );
            lines.push("  ret i64 %match_no_clause".to_string());
        } else {
            lines.push(format!(
                "  br i1 {condition}, label %bb{}, label %match_block{}_arm{}_next",
                arm.target, block.id, arm_index
            ));
            lines.push(format!("match_block{}_arm{}_next:", block.id, arm_index));
        }
    }

    Ok(())
}

fn hash_text_i64(value: &str) -> i64 {
    hash_bytes_i64(value.as_bytes())
}

fn hash_pattern_i64(pattern: &IrPattern) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(pattern).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize pattern hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn hash_ir_op_i64(op: &IrOp) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(op).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize ir op hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn hash_closure_descriptor_i64(
    params: &[String],
    ops: &[IrOp],
    capture_names: &[String],
) -> Result<i64, LlvmBackendError> {
    let serialized = serde_json::to_string(&(params, ops, capture_names)).map_err(|error| {
        LlvmBackendError::new(format!(
            "llvm backend failed to serialize closure descriptor hash input: {error}"
        ))
    })?;

    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn closure_capture_names(params: &[String], ops: &[IrOp]) -> Vec<String> {
    let mut captures = BTreeSet::new();
    let param_names = params.iter().cloned().collect::<BTreeSet<_>>();
    collect_capture_names_from_ops(ops, &param_names, &mut captures);
    captures.into_iter().collect()
}

fn collect_capture_names_from_ops(
    ops: &[IrOp],
    params: &BTreeSet<String>,
    captures: &mut BTreeSet<String>,
) {
    for op in ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if !params.contains(name) {
                    captures.insert(name.clone());
                }
            }
            IrOp::AndAnd { right_ops, .. }
            | IrOp::OrOr { right_ops, .. }
            | IrOp::And { right_ops, .. }
            | IrOp::Or { right_ops, .. } => {
                collect_capture_names_from_ops(right_ops, params, captures);
            }
            IrOp::Case { branches, .. } => {
                for branch in branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
            }
            IrOp::Try {
                body_ops,
                rescue_branches,
                catch_branches,
                after_ops,
                ..
            } => {
                collect_capture_names_from_ops(body_ops, params, captures);
                for branch in rescue_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                for branch in catch_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                if let Some(after_ops) = after_ops {
                    collect_capture_names_from_ops(after_ops, params, captures);
                }
            }
            IrOp::For {
                generators,
                into_ops,
                reduce_ops,
                body_ops,
                ..
            } => {
                for generator in generators {
                    collect_capture_names_from_ops(&generator.source_ops, params, captures);
                    if let Some(guard_ops) = &generator.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                }
                if let Some(into_ops) = into_ops {
                    collect_capture_names_from_ops(into_ops, params, captures);
                }
                if let Some(reduce_ops) = reduce_ops {
                    collect_capture_names_from_ops(reduce_ops, params, captures);
                }
                collect_capture_names_from_ops(body_ops, params, captures);
            }
            IrOp::MakeClosure { .. }
            | IrOp::ConstInt { .. }
            | IrOp::ConstFloat { .. }
            | IrOp::ConstBool { .. }
            | IrOp::ConstNil { .. }
            | IrOp::ConstString { .. }
            | IrOp::ToString { .. }
            | IrOp::Call { .. }
            | IrOp::CallValue { .. }
            | IrOp::Question { .. }
            | IrOp::Raise { .. }
            | IrOp::ConstAtom { .. }
            | IrOp::AddInt { .. }
            | IrOp::SubInt { .. }
            | IrOp::MulInt { .. }
            | IrOp::DivInt { .. }
            | IrOp::CmpInt { .. }
            | IrOp::Not { .. }
            | IrOp::Bang { .. }
            | IrOp::Concat { .. }
            | IrOp::In { .. }
            | IrOp::NotIn { .. }
            | IrOp::PlusPlus { .. }
            | IrOp::MinusMinus { .. }
            | IrOp::Range { .. }
            | IrOp::BitwiseAnd { .. }
            | IrOp::BitwiseOr { .. }
            | IrOp::BitwiseXor { .. }
            | IrOp::BitwiseNot { .. }
            | IrOp::BitwiseShiftLeft { .. }
            | IrOp::BitwiseShiftRight { .. }
            | IrOp::SteppedRange { .. }
            | IrOp::Match { .. }
            | IrOp::Return { .. } => {}
        }
    }
}

fn hash_bytes_i64(bytes: &[u8]) -> i64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    i64::from_ne_bytes(hash.to_ne_bytes())
}

fn emit_pattern_condition(
    _function_name: &str,
    operand: &str,
    pattern: &IrPattern,
    label: &str,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    match pattern {
        IrPattern::Wildcard => Ok("true".to_string()),
        IrPattern::Integer { value } => {
            let register = format!("%{label}_int");
            lines.push(format!("  {register} = icmp eq i64 {operand}, {value}"));
            Ok(register)
        }
        IrPattern::Bool { value } => {
            let register = format!("%{label}_bool");
            lines.push(format!(
                "  {register} = icmp eq i64 {operand}, {}",
                i64::from(*value)
            ));
            Ok(register)
        }
        IrPattern::Nil => {
            let register = format!("%{label}_nil");
            lines.push(format!("  {register} = icmp eq i64 {operand}, 0"));
            Ok(register)
        }
        _ => {
            let pattern_hash = hash_pattern_i64(pattern)?;
            let register = format!("%{label}_complex");
            lines.push(format!(
                "  {register} = call i1 @tn_runtime_pattern_matches(i64 {operand}, i64 {pattern_hash})"
            ));
            Ok(register)
        }
    }
}

fn emit_guard_condition(
    function_name: &str,
    guard_ops: &[IrOp],
    params: &[crate::mir::MirTypedName],
    label: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    let mut stack = Vec::<String>::new();

    for (index, op) in guard_ops.iter().enumerate() {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(param_index) = params.iter().position(|param| &param.name == name) {
                    stack.push(format!("%arg{param_index}"));
                } else {
                    let register = format!("%{label}_load_binding_{index}");
                    let binding_hash = hash_text_i64(name);
                    lines.push(format!(
                        "  {register} = call i64 @tn_runtime_load_binding(i64 {binding_hash})"
                    ));
                    stack.push(register);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let register = format!("%{label}_const_int_{index}");
                lines.push(format!("  {register} = add i64 0, {value}"));
                stack.push(register);
            }
            IrOp::ConstBool { value, .. } => {
                let register = format!("%{label}_const_bool_{index}");
                lines.push(format!("  {register} = add i64 0, {}", i64::from(*value)));
                stack.push(register);
            }
            IrOp::ConstNil { .. } => {
                let register = format!("%{label}_const_nil_{index}");
                lines.push(format!("  {register} = add i64 0, 0"));
                stack.push(register);
            }
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                if stack.len() < *argc {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    )));
                }

                let split_index = stack.len() - *argc;
                let call_args = stack.split_off(split_index);
                let rendered_args = call_args
                    .iter()
                    .map(|arg| format!("i64 {arg}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let result_register = format!("%{label}_call_{index}");
                match callee {
                    IrCallTarget::Function { name } => {
                        let target_key = (name.clone(), *argc);
                        if let Some(symbol) = callable_symbols.get(&target_key) {
                            lines.push(format!(
                                "  {result_register} = call i64 @{symbol}({rendered_args})"
                            ));
                        } else if callable_symbols
                            .keys()
                            .any(|(candidate, _)| candidate == name)
                        {
                            lines.push(format!(
                                "  {result_register} = call i64 @tn_runtime_error_arity_mismatch()"
                            ));
                        } else {
                            return Err(LlvmBackendError::new(format!(
                                "llvm backend unknown guard call target {name} in function {function_name} at offset {offset}"
                            )));
                        }
                    }
                    IrCallTarget::Builtin { name } => {
                        emit_builtin_call_from_registers(
                            result_register.clone(),
                            name,
                            call_args,
                            function_name,
                            *offset,
                            lines,
                        )?;
                    }
                }

                stack.push(result_register);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let left = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;

                let predicate = match kind {
                    CmpKind::Eq | CmpKind::StrictEq => "eq",
                    CmpKind::NotEq | CmpKind::StrictNotEq => "ne",
                    CmpKind::Lt => "slt",
                    CmpKind::Lte => "sle",
                    CmpKind::Gt => "sgt",
                    CmpKind::Gte => "sge",
                };

                let cmp_register = format!("%{label}_cmp_{index}");
                let cmp_value = format!("%{label}_cmp_value_{index}");
                lines.push(format!(
                    "  {cmp_register} = icmp {predicate} i64 {left}, {right}"
                ));
                lines.push(format!("  {cmp_value} = zext i1 {cmp_register} to i64"));
                stack.push(cmp_value);
            }
            IrOp::Bang { .. } => {
                let value = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let truthy = format!("%{label}_bang_truthy_{index}");
                let bang_value = format!("%{label}_bang_value_{index}");
                lines.push(format!("  {truthy} = icmp ne i64 {value}, 0"));
                lines.push(format!("  {bang_value} = zext i1 {truthy} to i64"));
                stack.push(bang_value);
            }
            IrOp::Not { .. } => {
                let value = stack.pop().ok_or_else(|| {
                    LlvmBackendError::new(format!(
                        "llvm backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let strict = format!("%{label}_not_strict_{index}");
                let not_value = format!("%{label}_not_value_{index}");
                lines.push(format!("  {strict} = icmp eq i64 {value}, 0"));
                lines.push(format!("  {not_value} = zext i1 {strict} to i64"));
                stack.push(not_value);
            }
            other => {
                return Err(LlvmBackendError::unsupported_guard_op(
                    function_name,
                    other,
                    0,
                ));
            }
        }
    }

    let Some(final_value) = stack.pop() else {
        return Err(LlvmBackendError::new(format!(
            "llvm backend guard stack underflow in function {function_name}"
        )));
    };

    if !stack.is_empty() {
        return Err(LlvmBackendError::new(format!(
            "llvm backend guard stack leftover values in function {function_name}"
        )));
    }

    let condition = format!("%{label}_truthy");
    lines.push(format!("  {condition} = icmp ne i64 {final_value}, 0"));
    Ok(condition)
}

fn combine_conditions(
    function_name: &str,
    conditions: Vec<String>,
    label: &str,
    lines: &mut Vec<String>,
) -> Result<String, LlvmBackendError> {
    if conditions.is_empty() {
        return Ok("true".to_string());
    }

    let mut iter = conditions.into_iter();
    let Some(mut current) = iter.next() else {
        return Err(LlvmBackendError::new(format!(
            "llvm backend missing condition in function {function_name}"
        )));
    };

    for (index, condition) in iter.enumerate() {
        let combined = format!("%{label}_and_{index}");
        lines.push(format!("  {combined} = and i1 {current}, {condition}"));
        current = combined;
    }

    Ok(current)
}

fn instruction_destination(instruction: &MirInstruction) -> Option<u32> {
    match instruction {
        MirInstruction::ConstInt { dest, .. }
        | MirInstruction::ConstFloat { dest, .. }
        | MirInstruction::ConstBool { dest, .. }
        | MirInstruction::ConstNil { dest, .. }
        | MirInstruction::ConstString { dest, .. }
        | MirInstruction::ConstAtom { dest, .. }
        | MirInstruction::LoadVariable { dest, .. }
        | MirInstruction::Unary { dest, .. }
        | MirInstruction::Binary { dest, .. }
        | MirInstruction::Call { dest, .. }
        | MirInstruction::CallValue { dest, .. }
        | MirInstruction::MakeClosure { dest, .. }
        | MirInstruction::Question { dest, .. }
        | MirInstruction::MatchPattern { dest, .. } => Some(*dest),
        MirInstruction::Legacy { dest, .. } => *dest,
    }
}

fn instruction_operands(instruction: &MirInstruction) -> Vec<u32> {
    match instruction {
        MirInstruction::Unary { input, .. } => vec![*input],
        MirInstruction::Binary { left, right, .. } => vec![*left, *right],
        MirInstruction::Call { args, .. } => args.clone(),
        MirInstruction::CallValue { callee, args, .. } => {
            let mut values = vec![*callee];
            values.extend(args.iter().copied());
            values
        }
        MirInstruction::Question { input, .. } => vec![*input],
        MirInstruction::MatchPattern { input, .. } => vec![*input],
        _ => Vec::new(),
    }
}

fn terminator_operands(terminator: &MirTerminator) -> Vec<u32> {
    match terminator {
        MirTerminator::Return { value, .. } => vec![*value],
        MirTerminator::Jump { args, .. } => args.clone(),
        MirTerminator::Match { scrutinee, .. } => vec![*scrutinee],
        MirTerminator::ShortCircuit { condition, .. } => vec![*condition],
    }
}
