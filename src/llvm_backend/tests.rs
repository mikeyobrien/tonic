use super::{lower_mir_subset_to_llvm_ir, LLVM_COMPATIBILITY_VERSION};
use crate::ir::IrCallTarget;
use crate::mir::{
    MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator, MirType,
    MirTypedName,
};

#[test]
fn lower_mir_subset_emits_deterministic_llvm_ir_for_int_bool_calls() {
    let mir = MirProgram {
        functions: vec![
            MirFunction {
                name: "Math.add".to_string(),
                params: vec![
                    MirTypedName {
                        name: "a".to_string(),
                        value_type: MirType::Dynamic,
                    },
                    MirTypedName {
                        name: "b".to_string(),
                        value_type: MirType::Dynamic,
                    },
                ],
                entry_block: 0,
                blocks: vec![MirBlock {
                    id: 0,
                    args: vec![],
                    instructions: vec![
                        MirInstruction::LoadVariable {
                            dest: 0,
                            name: "a".to_string(),
                            offset: 1,
                            value_type: MirType::Dynamic,
                        },
                        MirInstruction::LoadVariable {
                            dest: 1,
                            name: "b".to_string(),
                            offset: 2,
                            value_type: MirType::Dynamic,
                        },
                        MirInstruction::Binary {
                            dest: 2,
                            kind: MirBinaryKind::AddInt,
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
            },
            MirFunction {
                name: "Math.run".to_string(),
                params: vec![],
                entry_block: 0,
                blocks: vec![MirBlock {
                    id: 0,
                    args: vec![],
                    instructions: vec![
                        MirInstruction::ConstInt {
                            dest: 0,
                            value: 7,
                            offset: 11,
                            value_type: MirType::Int,
                        },
                        MirInstruction::ConstInt {
                            dest: 1,
                            value: 8,
                            offset: 12,
                            value_type: MirType::Int,
                        },
                        MirInstruction::Call {
                            dest: 2,
                            callee: IrCallTarget::Function {
                                name: "Math.add".to_string(),
                            },
                            args: vec![0, 1],
                            offset: 13,
                            value_type: MirType::Dynamic,
                        },
                        MirInstruction::ConstInt {
                            dest: 3,
                            value: 10,
                            offset: 14,
                            value_type: MirType::Int,
                        },
                        MirInstruction::Binary {
                            dest: 4,
                            kind: MirBinaryKind::CmpIntGt,
                            left: 2,
                            right: 3,
                            offset: 15,
                            value_type: MirType::Bool,
                        },
                    ],
                    terminator: MirTerminator::Return {
                        value: 4,
                        offset: 16,
                    },
                }],
            },
        ],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir).expect("subset MIR should lower to LLVM IR");

    assert!(llvm_ir.contains("; tonic llvm backend mvp"));
    assert!(llvm_ir.contains(&format!(
        "; llvm_compatibility={LLVM_COMPATIBILITY_VERSION}"
    )));
    assert!(llvm_ir.contains("define i64 @tn_Math_add(i64 %arg0, i64 %arg1)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_run()"));
    assert!(llvm_ir.contains("call i64 @tn_Math_add(i64 %v0, i64 %v1)"));
    assert!(llvm_ir.contains("icmp sgt i64 %v2, %v3"));
    assert!(llvm_ir.contains("zext i1 %cmp_4 to i64"));
}

#[test]
fn lower_mir_subset_rejects_out_of_subset_instruction() {
    let mir = MirProgram {
        functions: vec![MirFunction {
            name: "Unsupported.run".to_string(),
            params: vec![],
            entry_block: 0,
            blocks: vec![MirBlock {
                id: 0,
                args: vec![],
                instructions: vec![MirInstruction::ConstString {
                    dest: 0,
                    value: "hello".to_string(),
                    offset: 21,
                    value_type: MirType::String,
                }],
                terminator: MirTerminator::Return {
                    value: 0,
                    offset: 22,
                },
            }],
        }],
    };

    let error = lower_mir_subset_to_llvm_ir(&mir)
        .expect_err("const_string should be rejected by llvm mvp backend");

    assert_eq!(
        error.to_string(),
        "llvm backend unsupported instruction const_string in function Unsupported.run at offset 21"
    );
}
