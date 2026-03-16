use super::*;

pub(super) fn predecessor_edges(
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

pub(super) fn infer_block_arg_value_ids(
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

pub(super) fn block_external_value_ids(block: &MirBlock) -> Vec<u32> {
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

pub(super) fn unresolved_value_ids(function: &MirFunction) -> BTreeSet<u32> {
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

pub(super) fn emit_phi_nodes(
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
