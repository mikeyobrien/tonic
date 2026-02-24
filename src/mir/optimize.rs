use super::{MirBinaryKind, MirInstruction, MirProgram, MirTerminator, MirUnaryKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownValue {
    Int(i64),
    Bool(bool),
    Nil,
}

pub(crate) fn optimize_for_native_backend(mut program: MirProgram) -> MirProgram {
    for function in &mut program.functions {
        for block in &mut function.blocks {
            let mut known_values = HashMap::<u32, KnownValue>::new();

            for instruction in &mut block.instructions {
                fold_instruction(instruction, &mut known_values);
            }

            fold_terminator(&mut block.terminator, &known_values);
        }
    }

    program
}

fn fold_instruction(instruction: &mut MirInstruction, known_values: &mut HashMap<u32, KnownValue>) {
    if let Some((dest, folded)) = fold_binary_instruction(instruction, known_values) {
        *instruction = folded;
        known_values.insert(
            dest,
            infer_known_value(instruction).expect("folded value should be known"),
        );
        return;
    }

    if let Some((dest, folded)) = fold_unary_instruction(instruction, known_values) {
        *instruction = folded;
        known_values.insert(
            dest,
            infer_known_value(instruction).expect("folded value should be known"),
        );
        return;
    }

    if let Some(dest) = instruction_dest(instruction) {
        if let Some(value) = infer_known_value(instruction) {
            known_values.insert(dest, value);
        } else {
            known_values.remove(&dest);
        }
    }
}

fn fold_binary_instruction(
    instruction: &MirInstruction,
    known_values: &HashMap<u32, KnownValue>,
) -> Option<(u32, MirInstruction)> {
    let MirInstruction::Binary {
        dest,
        kind,
        left,
        right,
        offset,
        value_type: _,
    } = instruction
    else {
        return None;
    };

    let lhs = known_values.get(left)?;
    let rhs = known_values.get(right)?;

    let folded = match (kind, lhs, rhs) {
        (MirBinaryKind::AddInt, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstInt {
                dest: *dest,
                value: a.wrapping_add(*b),
                offset: *offset,
                value_type: crate::mir::MirType::Int,
            }
        }
        (MirBinaryKind::SubInt, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstInt {
                dest: *dest,
                value: a.wrapping_sub(*b),
                offset: *offset,
                value_type: crate::mir::MirType::Int,
            }
        }
        (MirBinaryKind::MulInt, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstInt {
                dest: *dest,
                value: a.wrapping_mul(*b),
                offset: *offset,
                value_type: crate::mir::MirType::Int,
            }
        }
        (MirBinaryKind::DivInt, KnownValue::Int(a), KnownValue::Int(b)) => {
            let value = a.checked_div(*b)?;
            MirInstruction::ConstInt {
                dest: *dest,
                value,
                offset: *offset,
                value_type: crate::mir::MirType::Int,
            }
        }
        (MirBinaryKind::CmpIntEq, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a == b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        (MirBinaryKind::CmpIntNotEq, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a != b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        (MirBinaryKind::CmpIntLt, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a < b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        (MirBinaryKind::CmpIntLte, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a <= b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        (MirBinaryKind::CmpIntGt, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a > b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        (MirBinaryKind::CmpIntGte, KnownValue::Int(a), KnownValue::Int(b)) => {
            MirInstruction::ConstBool {
                dest: *dest,
                value: a >= b,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            }
        }
        _ => return None,
    };

    Some((*dest, folded))
}

fn fold_unary_instruction(
    instruction: &MirInstruction,
    known_values: &HashMap<u32, KnownValue>,
) -> Option<(u32, MirInstruction)> {
    let MirInstruction::Unary {
        dest,
        kind,
        input,
        offset,
        value_type: _,
    } = instruction
    else {
        return None;
    };

    match (kind, known_values.get(input)?) {
        (MirUnaryKind::Not, KnownValue::Bool(value))
        | (MirUnaryKind::Bang, KnownValue::Bool(value)) => Some((
            *dest,
            MirInstruction::ConstBool {
                dest: *dest,
                value: !value,
                offset: *offset,
                value_type: crate::mir::MirType::Bool,
            },
        )),
        _ => None,
    }
}

fn fold_terminator(terminator: &mut MirTerminator, known_values: &HashMap<u32, KnownValue>) {
    let MirTerminator::ShortCircuit {
        op,
        condition,
        on_evaluate_rhs,
        on_short_circuit,
        offset: _,
    } = terminator
    else {
        return;
    };

    let Some(KnownValue::Bool(condition)) = known_values.get(condition) else {
        return;
    };

    let target = match op {
        crate::mir::MirShortCircuitOp::AndAnd | crate::mir::MirShortCircuitOp::And => {
            if *condition {
                *on_evaluate_rhs
            } else {
                *on_short_circuit
            }
        }
        crate::mir::MirShortCircuitOp::OrOr | crate::mir::MirShortCircuitOp::Or => {
            if *condition {
                *on_short_circuit
            } else {
                *on_evaluate_rhs
            }
        }
    };

    *terminator = MirTerminator::Jump {
        target,
        args: Vec::new(),
    };
}

fn infer_known_value(instruction: &MirInstruction) -> Option<KnownValue> {
    match instruction {
        MirInstruction::ConstInt { value, .. } => Some(KnownValue::Int(*value)),
        MirInstruction::ConstBool { value, .. } => Some(KnownValue::Bool(*value)),
        MirInstruction::ConstNil { .. } => Some(KnownValue::Nil),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::optimize_for_native_backend;
    use crate::mir::{
        MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator, MirType,
        MirTypedName,
    };

    #[test]
    fn folds_constant_int_binary_ops_into_const_values() {
        let program = MirProgram {
            functions: vec![MirFunction {
                name: "Demo.run".to_string(),
                params: vec![],
                param_patterns: None,
                guard_ops: None,
                entry_block: 0,
                blocks: vec![MirBlock {
                    id: 0,
                    args: vec![],
                    instructions: vec![
                        MirInstruction::ConstInt {
                            dest: 0,
                            value: 2,
                            offset: 1,
                            value_type: MirType::Int,
                        },
                        MirInstruction::ConstInt {
                            dest: 1,
                            value: 3,
                            offset: 2,
                            value_type: MirType::Int,
                        },
                        MirInstruction::Binary {
                            dest: 2,
                            kind: MirBinaryKind::MulInt,
                            left: 0,
                            right: 1,
                            offset: 3,
                            value_type: MirType::Int,
                        },
                    ],
                    terminator: MirTerminator::Return {
                        value: 2,
                        offset: 4,
                    },
                }],
            }],
        };

        let optimized = optimize_for_native_backend(program);
        let block = &optimized.functions[0].blocks[0];

        assert!(matches!(
            block.instructions[2],
            MirInstruction::ConstInt {
                dest: 2,
                value: 6,
                ..
            }
        ));
    }

    #[test]
    fn folds_bool_short_circuit_terminator_to_direct_jump() {
        let program = MirProgram {
            functions: vec![MirFunction {
                name: "Demo.run".to_string(),
                params: vec![MirTypedName {
                    name: "value".to_string(),
                    value_type: MirType::Bool,
                }],
                param_patterns: None,
                guard_ops: None,
                entry_block: 0,
                blocks: vec![
                    MirBlock {
                        id: 0,
                        args: vec![],
                        instructions: vec![MirInstruction::ConstBool {
                            dest: 0,
                            value: true,
                            offset: 1,
                            value_type: MirType::Bool,
                        }],
                        terminator: MirTerminator::ShortCircuit {
                            op: crate::mir::MirShortCircuitOp::AndAnd,
                            condition: 0,
                            on_evaluate_rhs: 1,
                            on_short_circuit: 2,
                            offset: 2,
                        },
                    },
                    MirBlock {
                        id: 1,
                        args: vec![],
                        instructions: vec![],
                        terminator: MirTerminator::Return {
                            value: 0,
                            offset: 3,
                        },
                    },
                    MirBlock {
                        id: 2,
                        args: vec![],
                        instructions: vec![],
                        terminator: MirTerminator::Return {
                            value: 0,
                            offset: 4,
                        },
                    },
                ],
            }],
        };

        let optimized = optimize_for_native_backend(program);
        let block = &optimized.functions[0].blocks[0];

        assert!(matches!(
            block.terminator,
            MirTerminator::Jump {
                target: 1,
                ref args
            } if args.is_empty()
        ));
    }

    #[test]
    fn does_not_fold_division_by_zero() {
        let program = MirProgram {
            functions: vec![MirFunction {
                name: "Demo.run".to_string(),
                params: vec![],
                param_patterns: None,
                guard_ops: None,
                entry_block: 0,
                blocks: vec![MirBlock {
                    id: 0,
                    args: vec![],
                    instructions: vec![
                        MirInstruction::ConstInt {
                            dest: 0,
                            value: 12,
                            offset: 1,
                            value_type: MirType::Int,
                        },
                        MirInstruction::ConstInt {
                            dest: 1,
                            value: 0,
                            offset: 2,
                            value_type: MirType::Int,
                        },
                        MirInstruction::Binary {
                            dest: 2,
                            kind: MirBinaryKind::DivInt,
                            left: 0,
                            right: 1,
                            offset: 3,
                            value_type: MirType::Int,
                        },
                    ],
                    terminator: MirTerminator::Return {
                        value: 2,
                        offset: 4,
                    },
                }],
            }],
        };

        let optimized = optimize_for_native_backend(program);
        let block = &optimized.functions[0].blocks[0];

        assert!(matches!(
            block.instructions[2],
            MirInstruction::Binary {
                kind: MirBinaryKind::DivInt,
                ..
            }
        ));
    }
}
