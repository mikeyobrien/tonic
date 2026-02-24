#[cfg(test)]
mod tests;

use crate::ir::IrCallTarget;
use crate::mir::{MirBinaryKind, MirInstruction, MirProgram, MirTerminator};
use std::collections::BTreeMap;
use std::fmt;

pub(crate) const LLVM_COMPATIBILITY_VERSION: &str = "18.1.8";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlvmBackendError {
    message: String,
}

impl LlvmBackendError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn unsupported_instruction(
        function: &str,
        instruction: &MirInstruction,
        offset: usize,
    ) -> Self {
        let op = instruction_name(instruction);
        Self::new(format!(
            "llvm backend unsupported instruction {op} in function {function} at offset {offset}"
        ))
    }

    fn unsupported_terminator(function: &str, terminator: &MirTerminator, offset: usize) -> Self {
        let kind = terminator_name(terminator);
        Self::new(format!(
            "llvm backend unsupported terminator {kind} in function {function} at offset {offset}"
        ))
    }
}

impl fmt::Display for LlvmBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LlvmBackendError {}

pub(crate) fn lower_mir_subset_to_llvm_ir(mir: &MirProgram) -> Result<String, LlvmBackendError> {
    let mut symbols = BTreeMap::new();
    for function in &mir.functions {
        symbols.insert(function.name.clone(), mangle_function_name(&function.name));
    }

    let mut lines = vec![
        "; tonic llvm backend mvp".to_string(),
        format!("; llvm_compatibility={LLVM_COMPATIBILITY_VERSION}"),
        "target triple = \"x86_64-unknown-linux-gnu\"".to_string(),
        String::new(),
    ];

    for function in &mir.functions {
        let symbol = symbols
            .get(&function.name)
            .expect("symbol table should include all function names");

        let params = function
            .params
            .iter()
            .enumerate()
            .map(|(index, _)| format!("i64 %arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");

        lines.push(format!("define i64 @{symbol}({params}) {{"));

        let Some(block) = function
            .blocks
            .iter()
            .find(|block| block.id == function.entry_block)
        else {
            return Err(LlvmBackendError::new(format!(
                "llvm backend missing entry block {} in function {}",
                function.entry_block, function.name
            )));
        };

        if !block.args.is_empty() {
            return Err(LlvmBackendError::unsupported_terminator(
                &function.name,
                &block.terminator,
                terminator_offset(&block.terminator),
            ));
        }

        lines.push("entry:".to_string());

        let mut values = BTreeMap::<u32, String>::new();

        for instruction in &block.instructions {
            match instruction {
                MirInstruction::ConstInt { dest, value, .. } => {
                    let name = value_register(*dest);
                    lines.push(format!("  {name} = add i64 0, {value}"));
                    values.insert(*dest, name);
                }
                MirInstruction::ConstBool { dest, value, .. } => {
                    let encoded = i64::from(*value);
                    let name = value_register(*dest);
                    lines.push(format!("  {name} = add i64 0, {encoded}"));
                    values.insert(*dest, name);
                }
                MirInstruction::LoadVariable {
                    dest, name, offset, ..
                } => {
                    let Some(param_index) =
                        function.params.iter().position(|param| &param.name == name)
                    else {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend unsupported load_variable {name} in function {} at offset {offset}",
                            function.name
                        )));
                    };

                    values.insert(*dest, format!("%arg{param_index}"));
                }
                MirInstruction::Binary {
                    dest,
                    kind,
                    left,
                    right,
                    offset,
                    ..
                } => {
                    let Some(left_operand) = values.get(left) else {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend missing lhs value %{left} in function {} at offset {offset}",
                            function.name
                        )));
                    };
                    let Some(right_operand) = values.get(right) else {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend missing rhs value %{right} in function {} at offset {offset}",
                            function.name
                        )));
                    };

                    match kind {
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

                            let name = value_register(*dest);
                            lines.push(format!(
                                "  {name} = {op} i64 {left_operand}, {right_operand}"
                            ));
                            values.insert(*dest, name);
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

                            let cmp_name = format!("%cmp_{dest}");
                            let value_name = value_register(*dest);
                            lines.push(format!(
                                "  {cmp_name} = icmp {predicate} i64 {left_operand}, {right_operand}"
                            ));
                            lines.push(format!("  {value_name} = zext i1 {cmp_name} to i64"));
                            values.insert(*dest, value_name);
                        }
                        _ => {
                            return Err(LlvmBackendError::unsupported_instruction(
                                &function.name,
                                instruction,
                                *offset,
                            ));
                        }
                    }
                }
                MirInstruction::Call {
                    dest,
                    callee,
                    args,
                    offset,
                    ..
                } => {
                    let IrCallTarget::Function { name } = callee else {
                        return Err(LlvmBackendError::unsupported_instruction(
                            &function.name,
                            instruction,
                            *offset,
                        ));
                    };

                    let Some(callee_symbol) = symbols.get(name) else {
                        return Err(LlvmBackendError::new(format!(
                            "llvm backend unknown function call target {name} in function {} at offset {offset}",
                            function.name
                        )));
                    };

                    let mut rendered_args = Vec::with_capacity(args.len());
                    for arg in args {
                        let Some(operand) = values.get(arg) else {
                            return Err(LlvmBackendError::new(format!(
                                "llvm backend missing call argument %{arg} in function {} at offset {offset}",
                                function.name
                            )));
                        };
                        rendered_args.push(format!("i64 {operand}"));
                    }

                    let name = value_register(*dest);
                    lines.push(format!(
                        "  {name} = call i64 @{callee_symbol}({})",
                        rendered_args.join(", ")
                    ));
                    values.insert(*dest, name);
                }
                _ => {
                    return Err(LlvmBackendError::unsupported_instruction(
                        &function.name,
                        instruction,
                        instruction_offset(instruction),
                    ));
                }
            }
        }

        match &block.terminator {
            MirTerminator::Return { value, .. } => {
                let Some(operand) = values.get(value) else {
                    return Err(LlvmBackendError::new(format!(
                        "llvm backend missing return value %{value} in function {}",
                        function.name
                    )));
                };
                lines.push(format!("  ret i64 {operand}"));
            }
            other => {
                return Err(LlvmBackendError::unsupported_terminator(
                    &function.name,
                    other,
                    terminator_offset(other),
                ));
            }
        }

        lines.push("}".to_string());
        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

pub(crate) fn write_llvm_artifacts(
    llvm_ir: &str,
    ll_path: &std::path::Path,
    object_path: &std::path::Path,
) -> Result<(), LlvmBackendError> {
    if let Err(error) = crate::cache::write_atomic(ll_path, llvm_ir) {
        return Err(LlvmBackendError::new(format!(
            "failed to write llvm ir artifact to {}: {}",
            ll_path.display(),
            error
        )));
    }

    let checksum = fnv1a64(llvm_ir.as_bytes());
    let object_payload = format!(
        "TONICOBJ\nllvm_compatibility={LLVM_COMPATIBILITY_VERSION}\nll_fnv1a64={checksum:016x}\n"
    );

    if let Err(error) = crate::cache::write_atomic(object_path, &object_payload) {
        return Err(LlvmBackendError::new(format!(
            "failed to write llvm object artifact to {}: {}",
            object_path.display(),
            error
        )));
    }

    Ok(())
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn mangle_function_name(name: &str) -> String {
    format!("tn_{}", sanitize_identifier(name))
}

fn sanitize_identifier(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn value_register(id: u32) -> String {
    format!("%v{id}")
}

fn instruction_name(instruction: &MirInstruction) -> &'static str {
    match instruction {
        MirInstruction::ConstInt { .. } => "const_int",
        MirInstruction::ConstFloat { .. } => "const_float",
        MirInstruction::ConstBool { .. } => "const_bool",
        MirInstruction::ConstNil { .. } => "const_nil",
        MirInstruction::ConstString { .. } => "const_string",
        MirInstruction::ConstAtom { .. } => "const_atom",
        MirInstruction::LoadVariable { .. } => "load_variable",
        MirInstruction::Unary { .. } => "unary",
        MirInstruction::Binary { .. } => "binary",
        MirInstruction::Call { .. } => "call",
        MirInstruction::CallValue { .. } => "call_value",
        MirInstruction::MakeClosure { .. } => "make_closure",
        MirInstruction::Question { .. } => "question",
        MirInstruction::MatchPattern { .. } => "match_pattern",
        MirInstruction::Legacy { .. } => "legacy",
    }
}

fn terminator_name(terminator: &MirTerminator) -> &'static str {
    match terminator {
        MirTerminator::Return { .. } => "return",
        MirTerminator::Jump { .. } => "jump",
        MirTerminator::Match { .. } => "match",
        MirTerminator::ShortCircuit { .. } => "short_circuit",
    }
}

fn instruction_offset(instruction: &MirInstruction) -> usize {
    match instruction {
        MirInstruction::ConstInt { offset, .. }
        | MirInstruction::ConstFloat { offset, .. }
        | MirInstruction::ConstBool { offset, .. }
        | MirInstruction::ConstNil { offset, .. }
        | MirInstruction::ConstString { offset, .. }
        | MirInstruction::ConstAtom { offset, .. }
        | MirInstruction::LoadVariable { offset, .. }
        | MirInstruction::Unary { offset, .. }
        | MirInstruction::Binary { offset, .. }
        | MirInstruction::Call { offset, .. }
        | MirInstruction::CallValue { offset, .. }
        | MirInstruction::MakeClosure { offset, .. }
        | MirInstruction::Question { offset, .. }
        | MirInstruction::MatchPattern { offset, .. }
        | MirInstruction::Legacy { offset, .. } => *offset,
    }
}

fn terminator_offset(terminator: &MirTerminator) -> usize {
    match terminator {
        MirTerminator::Return { offset, .. }
        | MirTerminator::Match { offset, .. }
        | MirTerminator::ShortCircuit { offset, .. } => *offset,
        MirTerminator::Jump { .. } => 0,
    }
}
