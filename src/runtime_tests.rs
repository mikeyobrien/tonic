use super::*;
use crate::ir::{IrCaseBranch, IrForGenerator, IrFunction};

fn make_program(functions: Vec<IrFunction>) -> IrProgram {
    IrProgram { functions }
}

#[test]
fn test_simple_return() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 42,
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(42)));
}

#[test]
fn test_add() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 2,
                offset: 0,
            },
            IrOp::AddInt { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(3)));
}

#[test]
fn test_missing_function() {
    let program = make_program(vec![]);
    assert!(evaluate_entrypoint(&program).is_err());
}

#[test]
fn test_function_call() {
    let program = make_program(vec![
        IrFunction {
            name: "Demo.run".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::Call {
                    callee: IrCallTarget::Function {
                        name: "Demo.helper".to_string(),
                    },
                    argc: 0,
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        },
        IrFunction {
            name: "Demo.helper".to_string(),
            params: vec![],
            param_patterns: None,
            guard_ops: None,
            ops: vec![
                IrOp::ConstInt {
                    value: 99,
                    offset: 0,
                },
                IrOp::Return { offset: 0 },
            ],
        },
    ]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(99)));
}

#[test]
fn test_load_variable() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec!["x".to_string()],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::LoadVariable {
                name: "x".to_string(),
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(
        evaluate_function(&program, "Demo.run", &[RuntimeValue::Int(7)], 0),
        Ok(RuntimeValue::Int(7))
    );
}

#[test]
fn test_case_basic() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::Case {
                branches: vec![
                    IrCaseBranch {
                        pattern: IrPattern::Integer { value: 1 },
                        guard_ops: None,
                        ops: vec![
                            IrOp::ConstAtom {
                                value: "one".to_string(),
                                offset: 0,
                            },
                            IrOp::Return { offset: 0 },
                        ],
                    },
                    IrCaseBranch {
                        pattern: IrPattern::Wildcard,
                        guard_ops: None,
                        ops: vec![
                            IrOp::ConstAtom {
                                value: "other".to_string(),
                                offset: 0,
                            },
                            IrOp::Return { offset: 0 },
                        ],
                    },
                ],
                offset: 0,
            },
        ],
    }]);
    assert_eq!(
        evaluate_entrypoint(&program),
        Ok(RuntimeValue::Atom("one".to_string()))
    );
}

#[test]
fn test_match_op() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 42,
                offset: 0,
            },
            IrOp::Match {
                pattern: IrPattern::Bind {
                    name: "x".to_string(),
                },
                offset: 0,
            },
            IrOp::LoadVariable {
                name: "x".to_string(),
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(42)));
}

#[test]
fn test_for_collect() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::For {
                generators: vec![IrForGenerator {
                    source_ops: vec![
                        IrOp::ConstInt {
                            value: 1,
                            offset: 0,
                        },
                        IrOp::ConstInt {
                            value: 3,
                            offset: 0,
                        },
                        IrOp::Range { offset: 0 },
                    ],
                    pattern: IrPattern::Bind {
                        name: "x".to_string(),
                    },
                    guard_ops: None,
                }],
                body_ops: vec![
                    IrOp::LoadVariable {
                        name: "x".to_string(),
                        offset: 0,
                    },
                    IrOp::ConstInt {
                        value: 2,
                        offset: 0,
                    },
                    IrOp::MulInt { offset: 0 },
                ],
                into_ops: None,
                reduce_ops: None,
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(
        evaluate_entrypoint(&program),
        Ok(RuntimeValue::List(vec![
            RuntimeValue::Int(2),
            RuntimeValue::Int(4),
            RuntimeValue::Int(6),
        ]))
    );
}

#[test]
fn test_try_rescue() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::Try {
                body_ops: vec![
                    IrOp::ConstString {
                        value: "boom".to_string(),
                        offset: 0,
                    },
                    IrOp::Raise { offset: 0 },
                ],
                rescue_branches: vec![IrCaseBranch {
                    pattern: IrPattern::Bind {
                        name: "e".to_string(),
                    },
                    guard_ops: None,
                    ops: vec![
                        IrOp::ConstAtom {
                            value: "rescued".to_string(),
                            offset: 0,
                        },
                        IrOp::Return { offset: 0 },
                    ],
                }],
                catch_branches: vec![],
                after_ops: None,
                offset: 0,
            },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(
        evaluate_entrypoint(&program),
        Ok(RuntimeValue::Atom("rescued".to_string()))
    );
}

#[test]
fn test_closure() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::MakeClosure {
                params: vec!["x".to_string()],
                ops: vec![
                    IrOp::LoadVariable {
                        name: "x".to_string(),
                        offset: 0,
                    },
                    IrOp::ConstInt {
                        value: 1,
                        offset: 0,
                    },
                    IrOp::AddInt { offset: 0 },
                    IrOp::Return { offset: 0 },
                ],
                offset: 0,
            },
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::CallValue { argc: 1, offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(6)));
}

#[test]
fn test_bitwise_and() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 3,
                offset: 0,
            },
            IrOp::BitwiseAnd { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(1)));
}

#[test]
fn test_bitwise_or() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 3,
                offset: 0,
            },
            IrOp::BitwiseOr { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(7)));
}

#[test]
fn test_bitwise_xor() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 6,
                offset: 0,
            },
            IrOp::BitwiseXor { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(3)));
}

#[test]
fn test_bitwise_not() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 5,
                offset: 0,
            },
            IrOp::BitwiseNot { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(-6)));
}

#[test]
fn test_bitwise_shift_left() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 1,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 4,
                offset: 0,
            },
            IrOp::BitwiseShiftLeft { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(16)));
}

#[test]
fn test_bitwise_shift_right() {
    let program = make_program(vec![IrFunction {
        name: "Demo.run".to_string(),
        params: vec![],
        param_patterns: None,
        guard_ops: None,
        ops: vec![
            IrOp::ConstInt {
                value: 16,
                offset: 0,
            },
            IrOp::ConstInt {
                value: 2,
                offset: 0,
            },
            IrOp::BitwiseShiftRight { offset: 0 },
            IrOp::Return { offset: 0 },
        ],
    }]);
    assert_eq!(evaluate_entrypoint(&program), Ok(RuntimeValue::Int(4)));
}
