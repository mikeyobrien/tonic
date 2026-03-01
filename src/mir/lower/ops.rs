use super::{pop_n, pop_stack, FunctionLowerer, StackEntry};
use crate::ir::{CmpKind, IrCallTarget, IrForGenerator, IrOp, IrPattern};
use crate::mir::{
    MirBinaryKind, MirBlock, MirInstruction, MirMatchArm, MirShortCircuitOp, MirTerminator,
    MirType, MirTypedName, MirUnaryKind,
};

pub(super) fn lower_op(
    op: &IrOp,
    lowerer: &mut FunctionLowerer,
) -> Result<(), crate::mir::MirLoweringError> {
    match op {
        IrOp::ConstInt { value, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstInt {
                dest,
                value: *value,
                offset: *offset,
                value_type: MirType::Int,
            });
            lowerer.push(StackEntry::new(dest, MirType::Int));
        }
        IrOp::ConstFloat { value, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstFloat {
                dest,
                value: value.clone(),
                offset: *offset,
                value_type: MirType::Float,
            });
            lowerer.push(StackEntry::new(dest, MirType::Float));
        }
        IrOp::ConstBool { value, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstBool {
                dest,
                value: *value,
                offset: *offset,
                value_type: MirType::Bool,
            });
            lowerer.push(StackEntry::new(dest, MirType::Bool));
        }
        IrOp::ConstNil { offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstNil {
                dest,
                offset: *offset,
                value_type: MirType::Nil,
            });
            lowerer.push(StackEntry::new(dest, MirType::Nil));
        }
        IrOp::ConstString { value, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstString {
                dest,
                value: value.clone(),
                offset: *offset,
                value_type: MirType::String,
            });
            lowerer.push(StackEntry::new(dest, MirType::String));
        }
        IrOp::ConstAtom { value, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::ConstAtom {
                dest,
                value: value.clone(),
                offset: *offset,
                value_type: MirType::Atom,
            });
            lowerer.push(StackEntry::new(dest, MirType::Atom));
        }
        IrOp::LoadVariable { name, offset } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::LoadVariable {
                dest,
                name: name.clone(),
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::ToString { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Unary {
                dest,
                kind: MirUnaryKind::ToString,
                input: input.id,
                offset: *offset,
                value_type: MirType::String,
            });
            lowerer.push(StackEntry::new(dest, MirType::String));
        }
        IrOp::Not { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Unary {
                dest,
                kind: MirUnaryKind::Not,
                input: input.id,
                offset: *offset,
                value_type: MirType::Bool,
            });
            lowerer.push(StackEntry::new(dest, MirType::Bool));
        }
        IrOp::Bang { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Unary {
                dest,
                kind: MirUnaryKind::Bang,
                input: input.id,
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::Raise { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Unary {
                dest,
                kind: MirUnaryKind::Raise,
                input: input.id,
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::BitwiseNot { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Unary {
                dest,
                kind: MirUnaryKind::BitwiseNot,
                input: input.id,
                offset: *offset,
                value_type: MirType::Int,
            });
            lowerer.push(StackEntry::new(dest, MirType::Int));
        }
        IrOp::AddInt { offset } => lower_binary_op(lowerer, MirBinaryKind::AddInt, MirType::Int, *offset)?,
        IrOp::SubInt { offset } => lower_binary_op(lowerer, MirBinaryKind::SubInt, MirType::Int, *offset)?,
        IrOp::MulInt { offset } => lower_binary_op(lowerer, MirBinaryKind::MulInt, MirType::Int, *offset)?,
        IrOp::DivInt { offset } => lower_binary_op(lowerer, MirBinaryKind::DivInt, MirType::Int, *offset)?,
        IrOp::Concat { offset } => lower_binary_op(lowerer, MirBinaryKind::Concat, MirType::Dynamic, *offset)?,
        IrOp::In { offset } => lower_binary_op(lowerer, MirBinaryKind::In, MirType::Bool, *offset)?,
        IrOp::NotIn { offset } => lower_binary_op(lowerer, MirBinaryKind::NotIn, MirType::Bool, *offset)?,
        IrOp::PlusPlus { offset } => lower_binary_op(lowerer, MirBinaryKind::PlusPlus, MirType::Dynamic, *offset)?,
        IrOp::MinusMinus { offset } => lower_binary_op(lowerer, MirBinaryKind::MinusMinus, MirType::Dynamic, *offset)?,
        IrOp::Range { offset } => lower_binary_op(lowerer, MirBinaryKind::Range, MirType::Dynamic, *offset)?,
        IrOp::BitwiseAnd { offset } => lower_binary_op(lowerer, MirBinaryKind::BitwiseAnd, MirType::Int, *offset)?,
        IrOp::BitwiseOr { offset } => lower_binary_op(lowerer, MirBinaryKind::BitwiseOr, MirType::Int, *offset)?,
        IrOp::BitwiseXor { offset } => lower_binary_op(lowerer, MirBinaryKind::BitwiseXor, MirType::Int, *offset)?,
        IrOp::BitwiseShiftLeft { offset } => lower_binary_op(lowerer, MirBinaryKind::BitwiseShiftLeft, MirType::Int, *offset)?,
        IrOp::BitwiseShiftRight { offset } => lower_binary_op(lowerer, MirBinaryKind::BitwiseShiftRight, MirType::Int, *offset)?,
        IrOp::SteppedRange { offset } => lower_binary_op(lowerer, MirBinaryKind::SteppedRange, MirType::Dynamic, *offset)?,
        IrOp::CmpInt { kind, offset } => {
            let mir_kind = match kind {
                CmpKind::Eq => MirBinaryKind::CmpIntEq,
                CmpKind::NotEq => MirBinaryKind::CmpIntNotEq,
                CmpKind::Lt => MirBinaryKind::CmpIntLt,
                CmpKind::Lte => MirBinaryKind::CmpIntLte,
                CmpKind::Gt => MirBinaryKind::CmpIntGt,
                CmpKind::Gte => MirBinaryKind::CmpIntGte,
                CmpKind::StrictEq => MirBinaryKind::CmpIntEq,
                CmpKind::StrictNotEq => MirBinaryKind::CmpIntNotEq,
            };
            lower_binary_op(lowerer, mir_kind, MirType::Bool, *offset)?;
        }
        IrOp::Call {
            callee,
            argc,
            offset,
        } => {
            let args = pop_n(lowerer, *argc, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Call {
                dest,
                callee: callee.clone(),
                args: args.iter().map(|e| e.id).collect(),
                offset: *offset,
                value_type: call_result_type(callee),
            });
            lowerer.push(StackEntry::new(dest, call_result_type(callee)));
        }
        IrOp::CallValue { argc, offset } => {
            let args = pop_n(lowerer, *argc, *offset)?;
            let callee = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::CallValue {
                dest,
                callee: callee.id,
                args: args.iter().map(|e| e.id).collect(),
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::MakeClosure {
            params,
            ops,
            offset,
        } => {
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::MakeClosure {
                dest,
                params: params.clone(),
                ops: ops.clone(),
                offset: *offset,
                value_type: MirType::Closure,
            });
            lowerer.push(StackEntry::new(dest, MirType::Closure));
        }
        IrOp::Question { offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Question {
                dest,
                input: input.id,
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::Match { pattern, offset } => {
            let input = pop_stack(lowerer, *offset)?;
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::MatchPattern {
                dest,
                input: input.id,
                pattern: pattern.clone(),
                offset: *offset,
                value_type: MirType::Dynamic,
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::Return { offset } => {
            let value = pop_stack(lowerer, *offset)?;
            lowerer.finish_block(MirTerminator::Return {
                value: value.id,
                offset: *offset,
            });
        }
        IrOp::AndAnd {
            right_ops,
            offset,
        } => lower_short_circuit(lowerer, MirShortCircuitOp::AndAnd, right_ops, *offset)?,
        IrOp::OrOr { right_ops, offset } => {
            lower_short_circuit(lowerer, MirShortCircuitOp::OrOr, right_ops, *offset)?;
        }
        IrOp::And { right_ops, offset } => {
            lower_short_circuit(lowerer, MirShortCircuitOp::And, right_ops, *offset)?;
        }
        IrOp::Or { right_ops, offset } => {
            lower_short_circuit(lowerer, MirShortCircuitOp::Or, right_ops, *offset)?;
        }
        IrOp::Case { branches, offset } => lower_case(lowerer, branches, None, *offset)?,
        IrOp::Try {
            body_ops,
            rescue_branches,
            catch_branches,
            after_ops,
            offset,
        } => {
            // Inline try as legacy op for now
            let dest = lowerer.next_id();
            lowerer.emit(MirInstruction::Legacy {
                dest: Some(dest),
                source: IrOp::Try {
                    body_ops: body_ops.clone(),
                    rescue_branches: rescue_branches.clone(),
                    catch_branches: catch_branches.clone(),
                    after_ops: after_ops.clone(),
                    offset: *offset,
                },
                offset: *offset,
                value_type: Some(MirType::Dynamic),
            });
            lowerer.push(StackEntry::new(dest, MirType::Dynamic));
        }
        IrOp::For {
            generators,
            into_ops,
            reduce_ops,
            body_ops,
            offset,
        } => {
            lower_for(lowerer, generators, into_ops, reduce_ops, body_ops, *offset)?;
        }
    }
    Ok(())
}

fn lower_binary_op(
    lowerer: &mut FunctionLowerer,
    kind: MirBinaryKind,
    result_type: MirType,
    offset: usize,
) -> Result<(), crate::mir::MirLoweringError> {
    let right = pop_stack(lowerer, offset)?;
    let left = pop_stack(lowerer, offset)?;
    let dest = lowerer.next_id();
    lowerer.emit(MirInstruction::Binary {
        dest,
        kind,
        left: left.id,
        right: right.id,
        offset,
        value_type: result_type,
    });
    lowerer.push(StackEntry::new(dest, result_type));
    Ok(())
}

fn lower_short_circuit(
    lowerer: &mut FunctionLowerer,
    op: MirShortCircuitOp,
    right_ops: &[IrOp],
    offset: usize,
) -> Result<(), crate::mir::MirLoweringError> {
    let condition = pop_stack(lowerer, offset)?;

    let rhs_block = lowerer.new_block();
    let merge_block = lowerer.new_block_with_arg(MirType::Dynamic);

    lowerer.finish_block(MirTerminator::ShortCircuit {
        op,
        condition: condition.id,
        on_evaluate_rhs: rhs_block,
        on_short_circuit: merge_block,
        offset,
    });

    // Emit RHS block
    lowerer.switch_to(rhs_block);
    for rhs_op in right_ops {
        lower_op(rhs_op, lowerer)?;
    }
    let rhs_result = pop_stack(lowerer, offset)?;
    lowerer.finish_block(MirTerminator::Jump {
        target: merge_block,
        args: vec![rhs_result.id],
    });

    // Continue in merge block
    lowerer.switch_to(merge_block);
    let merge_arg = lowerer.block_arg(merge_block);
    lowerer.push(StackEntry::new(merge_arg, MirType::Dynamic));

    Ok(())
}

fn lower_case(
    lowerer: &mut FunctionLowerer,
    branches: &[crate::ir::IrCaseBranch],
    subject_override: Option<u32>,
    offset: usize,
) -> Result<(), crate::mir::MirLoweringError> {
    let subject = match subject_override {
        Some(id) => StackEntry::new(id, MirType::Dynamic),
        None => pop_stack(lowerer, offset)?,
    };

    let merge_block = lowerer.new_block_with_arg(MirType::Dynamic);

    let arms = branches
        .iter()
        .map(|branch| {
            let arm_block = lowerer.new_block();
            MirMatchArm {
                pattern: branch.pattern.clone(),
                guard_ops: branch.guard_ops.clone(),
                target: arm_block,
            }
        })
        .collect::<Vec<_>>();

    lowerer.finish_block(MirTerminator::Match {
        scrutinee: subject.id,
        arms: arms.clone(),
        offset,
    });

    for (branch, arm) in branches.iter().zip(arms.iter()) {
        lowerer.switch_to(arm.target);
        for op in &branch.ops {
            lower_op(op, lowerer)?;
        }
        let result = pop_stack(lowerer, offset)?;
        lowerer.finish_block(MirTerminator::Jump {
            target: merge_block,
            args: vec![result.id],
        });
    }

    lowerer.switch_to(merge_block);
    let merge_arg = lowerer.block_arg(merge_block);
    lowerer.push(StackEntry::new(merge_arg, MirType::Dynamic));

    Ok(())
}

fn lower_for(
    lowerer: &mut FunctionLowerer,
    generators: &[IrForGenerator],
    into_ops: &Option<Vec<IrOp>>,
    reduce_ops: &Option<Vec<IrOp>>,
    body_ops: &[IrOp],
    offset: usize,
) -> Result<(), crate::mir::MirLoweringError> {
    // Inline for as legacy op
    let dest = lowerer.next_id();
    lowerer.emit(MirInstruction::Legacy {
        dest: Some(dest),
        source: IrOp::For {
            generators: generators.to_vec(),
            into_ops: into_ops.clone(),
            reduce_ops: reduce_ops.clone(),
            body_ops: body_ops.to_vec(),
            offset,
        },
        offset,
        value_type: Some(MirType::Dynamic),
    });
    lowerer.push(StackEntry::new(dest, MirType::Dynamic));
    Ok(())
}

fn call_result_type(callee: &IrCallTarget) -> MirType {
    match callee {
        IrCallTarget::Builtin { name } => match name.as_str() {
            "ok" | "err" => MirType::Result,
            "tuple" => MirType::Dynamic,
            "list" => MirType::Dynamic,
            "map" | "map_empty" | "map_put" | "map_update" | "map_access" => MirType::Dynamic,
            "keyword" | "keyword_append" => MirType::Dynamic,
            "protocol_dispatch" | "host_call" => MirType::Dynamic,
            "div" | "rem" => MirType::Int,
            "byte_size" | "bit_size" => MirType::Int,
            _ => MirType::Dynamic,
        },
        IrCallTarget::Function { .. } => MirType::Dynamic,
    }
}
