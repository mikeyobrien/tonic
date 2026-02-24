use super::{lower_mir_subset_to_llvm_ir, LLVM_COMPATIBILITY_VERSION};
use crate::ir::{CmpKind, IrCallTarget, IrCaseBranch, IrOp, IrPattern};
use crate::mir::{
    MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator, MirType,
    MirTypedName, MirUnaryKind,
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
fn lower_mir_subset_lowers_string_and_float_constants_to_runtime_helpers() {
    let mir = MirProgram {
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
                    MirInstruction::ConstString {
                        dest: 0,
                        value: "hello".to_string(),
                        offset: 21,
                        value_type: MirType::String,
                    },
                    MirInstruction::ConstFloat {
                        dest: 1,
                        value: "3.14".to_string(),
                        offset: 22,
                        value_type: MirType::Float,
                    },
                ],
                terminator: MirTerminator::Return {
                    value: 1,
                    offset: 23,
                },
            }],
        }],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir)
        .expect("const string/float should lower to runtime helpers");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_const_string(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_const_float(i64)"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_const_string(i64"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_const_float(i64"));
}

#[test]
fn lower_mir_subset_rejects_unsupported_legacy_instruction() {
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
                instructions: vec![MirInstruction::Legacy {
                    dest: None,
                    source: IrOp::For {
                        generators: vec![],
                        into_ops: None,
                        body_ops: vec![],
                        offset: 21,
                    },
                    offset: 21,
                    value_type: None,
                }],
                terminator: MirTerminator::Return {
                    value: 0,
                    offset: 22,
                },
            }],
        }],
    };

    let error = lower_mir_subset_to_llvm_ir(&mir)
        .expect_err("unsupported legacy instruction should keep deterministic diagnostics");

    assert_eq!(
        error.to_string(),
        "llvm backend unsupported instruction legacy in function Unsupported.run at offset 21"
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

#[test]
fn lower_mir_subset_emits_collection_and_pattern_runtime_helpers() {
    let mir = MirProgram {
        functions: vec![MirFunction {
            name: "Demo.classify".to_string(),
            params: vec![MirTypedName {
                name: "expected".to_string(),
                value_type: MirType::Dynamic,
            }],
            param_patterns: None,
            guard_ops: None,
            entry_block: 0,
            blocks: vec![MirBlock {
                id: 0,
                args: vec![],
                instructions: vec![
                    MirInstruction::ConstInt {
                        dest: 0,
                        value: 8,
                        offset: 10,
                        value_type: MirType::Int,
                    },
                    MirInstruction::LoadVariable {
                        dest: 1,
                        name: "expected".to_string(),
                        offset: 11,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::Call {
                        dest: 2,
                        callee: IrCallTarget::Builtin {
                            name: "list".to_string(),
                        },
                        args: vec![1, 0],
                        offset: 12,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::ConstAtom {
                        dest: 3,
                        value: "ok".to_string(),
                        offset: 13,
                        value_type: MirType::Atom,
                    },
                    MirInstruction::Call {
                        dest: 4,
                        callee: IrCallTarget::Builtin {
                            name: "map".to_string(),
                        },
                        args: vec![3, 2],
                        offset: 14,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::MatchPattern {
                        dest: 5,
                        input: 4,
                        pattern: crate::ir::IrPattern::Map {
                            entries: vec![crate::ir::IrMapPatternEntry {
                                key: crate::ir::IrPattern::Atom {
                                    value: "ok".to_string(),
                                },
                                value: crate::ir::IrPattern::List {
                                    items: vec![
                                        crate::ir::IrPattern::Pin {
                                            name: "expected".to_string(),
                                        },
                                        crate::ir::IrPattern::Bind {
                                            name: "value".to_string(),
                                        },
                                    ],
                                    tail: None,
                                },
                            }],
                        },
                        offset: 15,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::LoadVariable {
                        dest: 6,
                        name: "value".to_string(),
                        offset: 16,
                        value_type: MirType::Dynamic,
                    },
                ],
                terminator: MirTerminator::Return {
                    value: 6,
                    offset: 17,
                },
            }],
        }],
    };

    let llvm_ir =
        lower_mir_subset_to_llvm_ir(&mir).expect("collections and pattern helpers should lower");

    assert!(llvm_ir.contains("declare i64 (i64, ...) @tn_runtime_make_list"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_map(i64, i64)"));
    assert!(llvm_ir.contains("declare i1 @tn_runtime_pattern_matches(i64, i64)"));
    assert!(llvm_ir.contains("call i64 (i64, ...) @tn_runtime_make_list"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_map"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_const_atom"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_match_operator"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_load_binding"));
}

#[test]
fn lower_mir_subset_emits_error_flow_runtime_helpers() {
    let mir = MirProgram {
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
                    MirInstruction::ConstAtom {
                        dest: 0,
                        value: "ok".to_string(),
                        offset: 10,
                        value_type: MirType::Atom,
                    },
                    MirInstruction::Call {
                        dest: 1,
                        callee: IrCallTarget::Builtin {
                            name: "ok".to_string(),
                        },
                        args: vec![0],
                        offset: 11,
                        value_type: MirType::Result,
                    },
                    MirInstruction::ConstAtom {
                        dest: 2,
                        value: "boom".to_string(),
                        offset: 12,
                        value_type: MirType::Atom,
                    },
                    MirInstruction::Call {
                        dest: 3,
                        callee: IrCallTarget::Builtin {
                            name: "err".to_string(),
                        },
                        args: vec![2],
                        offset: 13,
                        value_type: MirType::Result,
                    },
                    MirInstruction::Question {
                        dest: 4,
                        input: 3,
                        offset: 14,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::Unary {
                        dest: 5,
                        kind: MirUnaryKind::Raise,
                        input: 2,
                        offset: 15,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::Legacy {
                        dest: Some(6),
                        source: IrOp::Try {
                            body_ops: vec![IrOp::ConstAtom {
                                value: "boom".to_string(),
                                offset: 16,
                            }],
                            rescue_branches: vec![IrCaseBranch {
                                pattern: IrPattern::Atom {
                                    value: "boom".to_string(),
                                },
                                guard_ops: None,
                                ops: vec![IrOp::ConstAtom {
                                    value: "rescued".to_string(),
                                    offset: 17,
                                }],
                            }],
                            catch_branches: vec![],
                            after_ops: None,
                            offset: 18,
                        },
                        offset: 18,
                        value_type: Some(MirType::Dynamic),
                    },
                ],
                terminator: MirTerminator::Return {
                    value: 6,
                    offset: 19,
                },
            }],
        }],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir).expect("error helpers should lower");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_ok(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_err(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_question(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_raise(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_try(i64)"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_ok"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_err"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_question"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_raise"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_try"));
}

#[test]
fn lower_mir_subset_emits_closure_runtime_helpers() {
    let mir = MirProgram {
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
                        value: 4,
                        offset: 10,
                        value_type: MirType::Int,
                    },
                    MirInstruction::MakeClosure {
                        dest: 1,
                        params: vec!["value".to_string()],
                        ops: vec![
                            IrOp::LoadVariable {
                                name: "value".to_string(),
                                offset: 11,
                            },
                            IrOp::LoadVariable {
                                name: "base".to_string(),
                                offset: 12,
                            },
                            IrOp::AddInt { offset: 13 },
                            IrOp::Return { offset: 14 },
                        ],
                        offset: 11,
                        value_type: MirType::Closure,
                    },
                    MirInstruction::ConstInt {
                        dest: 2,
                        value: 3,
                        offset: 15,
                        value_type: MirType::Int,
                    },
                    MirInstruction::CallValue {
                        dest: 3,
                        callee: 1,
                        args: vec![2],
                        offset: 16,
                        value_type: MirType::Dynamic,
                    },
                ],
                terminator: MirTerminator::Return {
                    value: 3,
                    offset: 17,
                },
            }],
        }],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir).expect("closure helpers should lower");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_closure(i64, i64, i64)"));
    assert!(llvm_ir.contains("declare i64 (i64, i64, ...) @tn_runtime_call_closure"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_closure"));
    assert!(llvm_ir.contains("i64 1, i64 1)"));
    assert!(llvm_ir.contains("call i64 (i64, i64, ...) @tn_runtime_call_closure"));
}

#[test]
fn lower_mir_subset_emits_host_interop_runtime_helpers() {
    let mir = MirProgram {
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
                    MirInstruction::ConstAtom {
                        dest: 0,
                        value: "sum_ints".to_string(),
                        offset: 10,
                        value_type: MirType::Atom,
                    },
                    MirInstruction::ConstInt {
                        dest: 1,
                        value: 20,
                        offset: 11,
                        value_type: MirType::Int,
                    },
                    MirInstruction::ConstInt {
                        dest: 2,
                        value: 22,
                        offset: 12,
                        value_type: MirType::Int,
                    },
                    MirInstruction::Call {
                        dest: 3,
                        callee: IrCallTarget::Builtin {
                            name: "host_call".to_string(),
                        },
                        args: vec![0, 1, 2],
                        offset: 13,
                        value_type: MirType::Dynamic,
                    },
                    MirInstruction::Call {
                        dest: 4,
                        callee: IrCallTarget::Builtin {
                            name: "protocol_dispatch".to_string(),
                        },
                        args: vec![3],
                        offset: 14,
                        value_type: MirType::Dynamic,
                    },
                ],
                terminator: MirTerminator::Return {
                    value: 4,
                    offset: 15,
                },
            }],
        }],
    };

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir).expect("host interop helpers should lower");

    assert!(llvm_ir.contains("declare i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_protocol_dispatch(i64)"));
    assert!(llvm_ir.contains("call i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_protocol_dispatch"));
}
