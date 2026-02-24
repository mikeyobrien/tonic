use super::{pop_n, pop_stack, FunctionLowerer, StackValue};
use crate::ir::{CmpKind, IrCallTarget, IrOp};
use crate::mir::{MirBinaryKind, MirInstruction, MirLoweringError, MirType, MirUnaryKind};

impl FunctionLowerer {
    pub(super) fn lower_linear_op(
        &mut self,
        block_id: u32,
        stack: &mut Vec<StackValue>,
        op: IrOp,
    ) -> Result<(), MirLoweringError> {
        match op {
            IrOp::ConstInt { value, offset } => {
                let dest = self.alloc_value(MirType::Int).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstInt {
                        dest,
                        value,
                        offset,
                        value_type: MirType::Int,
                    },
                    MirType::Int,
                )
            }
            IrOp::ConstFloat { value, offset } => {
                let dest = self.alloc_value(MirType::Float).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstFloat {
                        dest,
                        value,
                        offset,
                        value_type: MirType::Float,
                    },
                    MirType::Float,
                )
            }
            IrOp::ConstBool { value, offset } => {
                let dest = self.alloc_value(MirType::Bool).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstBool {
                        dest,
                        value,
                        offset,
                        value_type: MirType::Bool,
                    },
                    MirType::Bool,
                )
            }
            IrOp::ConstNil { offset } => {
                let dest = self.alloc_value(MirType::Nil).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstNil {
                        dest,
                        offset,
                        value_type: MirType::Nil,
                    },
                    MirType::Nil,
                )
            }
            IrOp::ConstString { value, offset } => {
                let dest = self.alloc_value(MirType::String).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstString {
                        dest,
                        value,
                        offset,
                        value_type: MirType::String,
                    },
                    MirType::String,
                )
            }
            IrOp::ConstAtom { value, offset } => {
                let dest = self.alloc_value(MirType::Atom).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::ConstAtom {
                        dest,
                        value,
                        offset,
                        value_type: MirType::Atom,
                    },
                    MirType::Atom,
                )
            }
            IrOp::LoadVariable { name, offset } => {
                let dest = self.alloc_value(MirType::Dynamic).id;
                self.push_const(
                    block_id,
                    stack,
                    MirInstruction::LoadVariable {
                        dest,
                        name,
                        offset,
                        value_type: MirType::Dynamic,
                    },
                    MirType::Dynamic,
                )
            }
            IrOp::ToString { offset } => self.push_unary(
                block_id,
                stack,
                MirUnaryKind::ToString,
                offset,
                MirType::String,
            ),
            IrOp::Not { offset } => {
                self.push_unary(block_id, stack, MirUnaryKind::Not, offset, MirType::Bool)
            }
            IrOp::Bang { offset } => {
                self.push_unary(block_id, stack, MirUnaryKind::Bang, offset, MirType::Bool)
            }
            IrOp::Raise { offset } => self.push_unary(
                block_id,
                stack,
                MirUnaryKind::Raise,
                offset,
                MirType::Dynamic,
            ),
            IrOp::AddInt { offset } => {
                self.push_binary(block_id, stack, MirBinaryKind::AddInt, offset, MirType::Int)
            }
            IrOp::SubInt { offset } => {
                self.push_binary(block_id, stack, MirBinaryKind::SubInt, offset, MirType::Int)
            }
            IrOp::MulInt { offset } => {
                self.push_binary(block_id, stack, MirBinaryKind::MulInt, offset, MirType::Int)
            }
            IrOp::DivInt { offset } => {
                self.push_binary(block_id, stack, MirBinaryKind::DivInt, offset, MirType::Int)
            }
            IrOp::CmpInt { kind, offset } => {
                let binary_kind = match kind {
                    CmpKind::Eq => MirBinaryKind::CmpIntEq,
                    CmpKind::NotEq => MirBinaryKind::CmpIntNotEq,
                    CmpKind::Lt => MirBinaryKind::CmpIntLt,
                    CmpKind::Lte => MirBinaryKind::CmpIntLte,
                    CmpKind::Gt => MirBinaryKind::CmpIntGt,
                    CmpKind::Gte => MirBinaryKind::CmpIntGte,
                };
                self.push_binary(block_id, stack, binary_kind, offset, MirType::Bool)
            }
            IrOp::Concat { offset } => self.push_binary(
                block_id,
                stack,
                MirBinaryKind::Concat,
                offset,
                MirType::String,
            ),
            IrOp::In { offset } => {
                self.push_binary(block_id, stack, MirBinaryKind::In, offset, MirType::Bool)
            }
            IrOp::PlusPlus { offset } => self.push_binary(
                block_id,
                stack,
                MirBinaryKind::PlusPlus,
                offset,
                MirType::Dynamic,
            ),
            IrOp::MinusMinus { offset } => self.push_binary(
                block_id,
                stack,
                MirBinaryKind::MinusMinus,
                offset,
                MirType::Dynamic,
            ),
            IrOp::Range { offset } => self.push_binary(
                block_id,
                stack,
                MirBinaryKind::Range,
                offset,
                MirType::Dynamic,
            ),
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                let args = pop_n(stack, argc, "call arguments")?;
                let value = self.alloc_value(infer_call_type(&callee));
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::Call {
                        dest: value.id,
                        callee,
                        args: args.into_iter().map(|value| value.id).collect(),
                        offset,
                        value_type: value.value_type,
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::CallValue { argc, offset } => {
                let args = pop_n(stack, argc, "call value arguments")?;
                let callee = pop_stack(stack, "call value callee")?;
                let value = self.alloc_value(MirType::Dynamic);
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::CallValue {
                        dest: value.id,
                        callee: callee.id,
                        args: args.into_iter().map(|value| value.id).collect(),
                        offset,
                        value_type: value.value_type,
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::MakeClosure {
                params,
                ops,
                offset,
            } => {
                let value = self.alloc_value(MirType::Closure);
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::MakeClosure {
                        dest: value.id,
                        params,
                        ops,
                        offset,
                        value_type: value.value_type,
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::Question { offset } => {
                let input = pop_stack(stack, "question input")?;
                let value = self.alloc_value(MirType::Dynamic);
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::Question {
                        dest: value.id,
                        input: input.id,
                        offset,
                        value_type: value.value_type,
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::Match { pattern, offset } => {
                let input = pop_stack(stack, "match input")?;
                let value = self.alloc_value(MirType::Dynamic);
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::MatchPattern {
                        dest: value.id,
                        input: input.id,
                        pattern,
                        offset,
                        value_type: value.value_type,
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::Try { offset, .. } | IrOp::For { offset, .. } => {
                let value = self.alloc_value(MirType::Dynamic);
                self.block_mut(block_id)
                    .instructions
                    .push(MirInstruction::Legacy {
                        dest: Some(value.id),
                        source: op,
                        offset,
                        value_type: Some(value.value_type),
                    });
                stack.push(value);
                Ok(())
            }
            IrOp::Return { .. }
            | IrOp::Case { .. }
            | IrOp::AndAnd { .. }
            | IrOp::OrOr { .. }
            | IrOp::And { .. }
            | IrOp::Or { .. } => unreachable!(),
        }
    }

    fn push_const(
        &mut self,
        block_id: u32,
        stack: &mut Vec<StackValue>,
        instruction: MirInstruction,
        value_type: MirType,
    ) -> Result<(), MirLoweringError> {
        let Some(dest) = instruction_dest(&instruction) else {
            return Err(MirLoweringError::new(
                "const instruction missing destination",
            ));
        };

        self.block_mut(block_id).instructions.push(instruction);
        stack.push(StackValue {
            id: dest,
            value_type,
        });
        Ok(())
    }

    fn push_unary(
        &mut self,
        block_id: u32,
        stack: &mut Vec<StackValue>,
        kind: MirUnaryKind,
        offset: usize,
        result_type: MirType,
    ) -> Result<(), MirLoweringError> {
        let input = pop_stack(stack, "unary input")?;
        let value = self.alloc_value(result_type);
        self.block_mut(block_id)
            .instructions
            .push(MirInstruction::Unary {
                dest: value.id,
                kind,
                input: input.id,
                offset,
                value_type: value.value_type,
            });
        stack.push(value);
        Ok(())
    }

    fn push_binary(
        &mut self,
        block_id: u32,
        stack: &mut Vec<StackValue>,
        kind: MirBinaryKind,
        offset: usize,
        result_type: MirType,
    ) -> Result<(), MirLoweringError> {
        let right = pop_stack(stack, "binary rhs")?;
        let left = pop_stack(stack, "binary lhs")?;
        let value = self.alloc_value(result_type);
        self.block_mut(block_id)
            .instructions
            .push(MirInstruction::Binary {
                dest: value.id,
                kind,
                left: left.id,
                right: right.id,
                offset,
                value_type: value.value_type,
            });
        stack.push(value);
        Ok(())
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

fn infer_call_type(callee: &IrCallTarget) -> MirType {
    match callee {
        IrCallTarget::Builtin { name } if name == "ok" || name == "err" => MirType::Result,
        IrCallTarget::Builtin { name } if name == "to_string" => MirType::String,
        _ => MirType::Dynamic,
    }
}
