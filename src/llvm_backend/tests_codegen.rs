use super::lower_mir_subset_to_llvm_ir;
use crate::ir::{IrCallTarget, IrCaseBranch, IrOp, IrPattern};
use crate::mir::{
    MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator, MirType, MirTypedName,
    MirUnaryKind,
};
use crate::target::TargetTriple;

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

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir, &TargetTriple::host())
        .expect("collections and pattern helpers should lower");

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

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir, &TargetTriple::host())
        .expect("error helpers should lower");

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

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir, &TargetTriple::host())
        .expect("closure helpers should lower");

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

    let llvm_ir = lower_mir_subset_to_llvm_ir(&mir, &TargetTriple::host())
        .expect("host interop helpers should lower");

    assert!(llvm_ir.contains("declare i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_protocol_dispatch(i64)"));
    assert!(llvm_ir.contains("call i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_protocol_dispatch"));
}
