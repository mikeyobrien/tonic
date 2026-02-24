use crate::mir::{MirFunction, MirInstruction, MirTerminator};
use std::collections::{BTreeMap, BTreeSet};

use super::error::CBackendError;
use super::ops::emit_c_instructions;
use super::terminator::emit_c_terminator_with_phi;

pub(super) fn emit_function(
    function: &MirFunction,
    symbol: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(i, _)| format!("TnVal _arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!("static TnVal {symbol}({params}) {{\n"));

    // Infer which register IDs correspond to block-arg (phi) slots.
    // These are registers used in a block but not produced by its instructions.
    let phi_ids = infer_block_phi_reg_ids(function);

    // Declare ALL registers as locals at the function top.  This must include
    // both instruction destinations and block-arg (phi) registers, because C
    // forbids jumping past variable declarations with `goto`.
    let mut all_regs = collect_all_dests(function);
    for ids in phi_ids.values() {
        for id in ids {
            if !all_regs.contains(id) {
                all_regs.push(*id);
            }
        }
    }
    all_regs.sort_unstable();

    if !all_regs.is_empty() {
        let decls = all_regs
            .iter()
            .map(|id| format!("v{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  TnVal {decls};\n"));
    }

    for block in &function.blocks {
        out.push_str(&format!("  bb{}: ;\n", block.id));
        emit_c_instructions(function, block, callable_symbols, out)?;
        emit_c_terminator_with_phi(function, block, &phi_ids, callable_symbols, out)?;
    }

    out.push_str("}\n\n");
    Ok(())
}

fn collect_all_dests(function: &MirFunction) -> Vec<u32> {
    let mut dests = Vec::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Some(dest) = instruction_dest(instruction) {
                if !dests.contains(&dest) {
                    dests.push(dest);
                }
            }
        }
    }
    dests.sort_unstable();
    dests
}

fn instruction_dest(instruction: &MirInstruction) -> Option<u32> {
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
        MirInstruction::ConstInt { .. }
        | MirInstruction::ConstFloat { .. }
        | MirInstruction::ConstBool { .. }
        | MirInstruction::ConstNil { .. }
        | MirInstruction::ConstString { .. }
        | MirInstruction::ConstAtom { .. } => vec![],
        MirInstruction::LoadVariable { .. } => vec![],
        MirInstruction::Unary { input, .. } | MirInstruction::Question { input, .. } => {
            vec![*input]
        }
        MirInstruction::Binary { left, right, .. } => vec![*left, *right],
        MirInstruction::Call { args, .. } => args.clone(),
        MirInstruction::CallValue { callee, args, .. } => {
            let mut ops = vec![*callee];
            ops.extend(args);
            ops
        }
        MirInstruction::MakeClosure { .. } => vec![],
        MirInstruction::MatchPattern { input, .. } => vec![*input],
        MirInstruction::Legacy { .. } => vec![],
    }
}

fn terminator_operands(terminator: &MirTerminator) -> Vec<u32> {
    match terminator {
        MirTerminator::Return { value, .. } => vec![*value],
        MirTerminator::Jump { args, .. } => args.clone(),
        MirTerminator::ShortCircuit { condition, .. } => vec![*condition],
        MirTerminator::Match { scrutinee, .. } => vec![*scrutinee],
    }
}

/// Infer, for each block that has args, the register IDs that serve as phi
/// inputs (values that are used in the block but not produced by its instructions).
///
/// This mirrors the logic in `llvm_backend::codegen::infer_block_arg_value_ids`.
fn infer_block_phi_reg_ids(function: &MirFunction) -> BTreeMap<u32, Vec<u32>> {
    let mut result = BTreeMap::new();

    for block in &function.blocks {
        if block.args.is_empty() {
            result.insert(block.id, Vec::new());
            continue;
        }

        let mut defined = BTreeSet::<u32>::new();
        let mut ordered_external = Vec::<u32>::new();

        for instruction in &block.instructions {
            for used in instruction_operands(instruction) {
                if !defined.contains(&used) && !ordered_external.contains(&used) {
                    ordered_external.push(used);
                }
            }
            if let Some(dest) = instruction_dest(instruction) {
                defined.insert(dest);
            }
        }

        for used in terminator_operands(&block.terminator) {
            if !defined.contains(&used) && !ordered_external.contains(&used) {
                ordered_external.push(used);
            }
        }

        result.insert(block.id, ordered_external);
    }

    result
}
