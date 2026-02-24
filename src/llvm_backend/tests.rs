use super::{lower_mir_subset_to_llvm_ir, LLVM_COMPATIBILITY_VERSION};
use crate::ir::{CmpKind, IrCallTarget, IrOp};
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
                param_patterns: None,
                guard_ops: None,
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
                param_patterns: None,
                guard_ops: None,
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
    assert!(llvm_ir.contains("define i64 @tn_Math_add__arity2(i64 %arg0, i64 %arg1)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_run__arity0()"));
    assert!(llvm_ir.contains("call i64 @tn_Math_add__arity2(i64 %v0, i64 %v1)"));
    assert!(llvm_ir.contains("icmp sgt i64 %v2, %v3"));
    assert!(llvm_ir.contains("zext i1 %cmp_4 to i64"));
    assert!(llvm_ir.contains("define i64 @main()"));
}

#[test]
fn lower_mir_subset_rejects_out_of_subset_instruction() {
    let mir = MirProgram {
        functions: vec![MirFunction {
            name: "Unsupported.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
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

#[test]
fn lower_mir_subset_emits_control_flow_blocks_for_jump_terminators() {
    let mir = MirProgram {
        functions: vec![MirFunction {
            name: "Demo.run".to_string(),
            params: vec![],
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
                    terminator: MirTerminator::Jump {
                        target: 1,
                        args: vec![0],
                    },
                },
                MirBlock {
                    id: 1,
                    args: vec![MirTypedName {
                        name: "b1_arg0".to_string(),
                        value_type: MirType::Dynamic,
                    }],
                    instructions: vec![],
                    terminator: MirTerminator::Return {
                        value: 1,
                        offset: 2,
                    },
                },
            ],
        }],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir).expect("jump lowering should succeed");

    assert!(llvm_ir.contains("bb0:"));
    assert!(llvm_ir.contains("br label %bb1"));
    assert!(llvm_ir.contains("bb1:"));
    assert!(llvm_ir.contains("phi i64 [ %v0, %bb0 ]"));
}

#[test]
fn lower_mir_subset_emits_dispatcher_symbols_for_duplicate_function_clauses() {
    let clause_body = vec![MirBlock {
        id: 0,
        args: vec![],
        instructions: vec![MirInstruction::LoadVariable {
            dest: 0,
            name: "value".to_string(),
            offset: 10,
            value_type: MirType::Dynamic,
        }],
        terminator: MirTerminator::Return {
            value: 0,
            offset: 11,
        },
    }];

    let mir = MirProgram {
        functions: vec![
            MirFunction {
                name: "Demo.choose".to_string(),
                params: vec![MirTypedName {
                    name: "value".to_string(),
                    value_type: MirType::Dynamic,
                }],
                param_patterns: None,
                guard_ops: Some(vec![
                    IrOp::LoadVariable {
                        name: "value".to_string(),
                        offset: 30,
                    },
                    IrOp::ConstInt {
                        value: 10,
                        offset: 31,
                    },
                    IrOp::CmpInt {
                        kind: CmpKind::Gt,
                        offset: 32,
                    },
                ]),
                entry_block: 0,
                blocks: clause_body.clone(),
            },
            MirFunction {
                name: "Demo.choose".to_string(),
                params: vec![MirTypedName {
                    name: "value".to_string(),
                    value_type: MirType::Dynamic,
                }],
                param_patterns: None,
                guard_ops: None,
                entry_block: 0,
                blocks: clause_body,
            },
            MirFunction {
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
                            value: 7,
                            offset: 20,
                            value_type: MirType::Int,
                        },
                        MirInstruction::Call {
                            dest: 1,
                            callee: IrCallTarget::Function {
                                name: "Demo.choose".to_string(),
                            },
                            args: vec![0],
                            offset: 21,
                            value_type: MirType::Dynamic,
                        },
                    ],
                    terminator: MirTerminator::Return {
                        value: 1,
                        offset: 22,
                    },
                }],
            },
        ],
    };

    let llvm_ir =
        lower_mir_subset_to_llvm_ir(&mir).expect("duplicate clause lowering should succeed");

    assert!(llvm_ir.contains("define i64 @tn_Demo_choose__arity1(i64 %arg0)"));
    assert!(llvm_ir.contains("define i64 @tn_Demo_choose__arity1__clause0(i64 %arg0)"));
    assert!(llvm_ir.contains("define i64 @tn_Demo_choose__arity1__clause1(i64 %arg0)"));
    assert!(llvm_ir.contains("call i64 @tn_Demo_choose__arity1(i64 %v0)"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_error_no_matching_clause()"));
}
