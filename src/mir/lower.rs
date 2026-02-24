mod ops;

use super::{
    MirBlock, MirFunction, MirLoweringError, MirMatchArm, MirProgram, MirShortCircuitOp,
    MirTerminator, MirType, MirTypedName,
};
use crate::ir::{IrCaseBranch, IrOp, IrProgram};

pub(crate) fn lower_ir_to_mir_impl(ir: &IrProgram) -> Result<MirProgram, MirLoweringError> {
    let mut functions = Vec::with_capacity(ir.functions.len());

    for function in &ir.functions {
        let lowerer = FunctionLowerer::new(
            function.name.clone(),
            function.params.clone(),
            function.param_patterns.clone(),
            function.guard_ops.clone(),
        );
        functions.push(lowerer.lower(&function.ops)?);
    }

    Ok(MirProgram { functions })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StackValue {
    pub(super) id: u32,
    pub(super) value_type: MirType,
}

pub(super) struct BlockBuilder {
    pub(super) id: u32,
    pub(super) args: Vec<MirTypedName>,
    pub(super) instructions: Vec<super::MirInstruction>,
    pub(super) terminator: Option<MirTerminator>,
}

pub(super) struct FunctionLowerer {
    pub(super) name: String,
    pub(super) params: Vec<MirTypedName>,
    pub(super) param_patterns: Option<Vec<crate::ir::IrPattern>>,
    pub(super) guard_ops: Option<Vec<IrOp>>,
    pub(super) blocks: Vec<BlockBuilder>,
    pub(super) next_value: u32,
}

impl FunctionLowerer {
    fn new(
        name: String,
        params: Vec<String>,
        param_patterns: Option<Vec<crate::ir::IrPattern>>,
        guard_ops: Option<Vec<IrOp>>,
    ) -> Self {
        let mut lowerer = Self {
            name,
            params: params
                .into_iter()
                .map(|name| MirTypedName {
                    name,
                    value_type: MirType::Dynamic,
                })
                .collect(),
            param_patterns,
            guard_ops,
            blocks: Vec::new(),
            next_value: 0,
        };
        lowerer.create_block(Vec::new());
        lowerer
    }

    fn lower(mut self, ops: &[IrOp]) -> Result<MirFunction, MirLoweringError> {
        let (_, stack) = self.lower_ops(0, Vec::new(), ops)?;

        if !stack.is_empty() {
            return Err(MirLoweringError::new(format!(
                "unterminated MIR stack in function {}",
                self.name
            )));
        }

        let mut blocks = Vec::with_capacity(self.blocks.len());
        for block in self.blocks {
            let Some(terminator) = block.terminator else {
                return Err(MirLoweringError::new(format!(
                    "block {} in function {} has no terminator",
                    block.id, self.name
                )));
            };

            blocks.push(MirBlock {
                id: block.id,
                args: block.args,
                instructions: block.instructions,
                terminator,
            });
        }

        Ok(MirFunction {
            name: self.name,
            params: self.params,
            param_patterns: self.param_patterns,
            guard_ops: self.guard_ops,
            entry_block: 0,
            blocks,
        })
    }

    fn lower_ops(
        &mut self,
        mut block_id: u32,
        mut stack: Vec<StackValue>,
        ops: &[IrOp],
    ) -> Result<(u32, Vec<StackValue>), MirLoweringError> {
        for op in ops {
            if self.block(block_id).terminator.is_some() {
                return Err(MirLoweringError::new(format!(
                    "block {} already terminated while lowering {}",
                    block_id, self.name
                )));
            }

            match op {
                IrOp::Return { offset } => {
                    let value = pop_stack(&mut stack, "return value")?;
                    self.set_terminator(
                        block_id,
                        MirTerminator::Return {
                            value: value.id,
                            offset: *offset,
                        },
                    )?;
                }
                IrOp::Case { branches, offset } => {
                    (block_id, stack) = self.lower_case(block_id, stack, branches, *offset)?;
                }
                IrOp::AndAnd { right_ops, offset } => {
                    (block_id, stack) = self.lower_short_circuit(
                        block_id,
                        stack,
                        right_ops,
                        MirShortCircuitOp::AndAnd,
                        *offset,
                    )?;
                }
                IrOp::OrOr { right_ops, offset } => {
                    (block_id, stack) = self.lower_short_circuit(
                        block_id,
                        stack,
                        right_ops,
                        MirShortCircuitOp::OrOr,
                        *offset,
                    )?;
                }
                IrOp::And { right_ops, offset } => {
                    (block_id, stack) = self.lower_short_circuit(
                        block_id,
                        stack,
                        right_ops,
                        MirShortCircuitOp::And,
                        *offset,
                    )?;
                }
                IrOp::Or { right_ops, offset } => {
                    (block_id, stack) = self.lower_short_circuit(
                        block_id,
                        stack,
                        right_ops,
                        MirShortCircuitOp::Or,
                        *offset,
                    )?;
                }
                other => self.lower_linear_op(block_id, &mut stack, other.clone())?,
            }
        }

        Ok((block_id, stack))
    }

    fn lower_case(
        &mut self,
        block_id: u32,
        mut stack: Vec<StackValue>,
        branches: &[IrCaseBranch],
        offset: usize,
    ) -> Result<(u32, Vec<StackValue>), MirLoweringError> {
        let scrutinee = pop_stack(&mut stack, "case scrutinee")?;
        let branch_blocks = (0..branches.len())
            .map(|_| self.create_block(Vec::new()))
            .collect::<Vec<_>>();
        let merge_block = self.create_block(vec![MirType::Dynamic]);
        let merge_value = self.alloc_value(MirType::Dynamic);

        let arms = branches
            .iter()
            .zip(branch_blocks.iter().copied())
            .map(|(branch, target)| MirMatchArm {
                pattern: branch.pattern.clone(),
                guard_ops: branch.guard_ops.clone(),
                target,
            })
            .collect::<Vec<_>>();

        self.set_terminator(
            block_id,
            MirTerminator::Match {
                scrutinee: scrutinee.id,
                arms,
                offset,
            },
        )?;

        for (branch, branch_block) in branches.iter().zip(branch_blocks.iter().copied()) {
            let (end_block, mut branch_stack) =
                self.lower_ops(branch_block, Vec::new(), &branch.ops)?;
            if self.block(end_block).terminator.is_some() {
                return Err(MirLoweringError::new("case branch unexpectedly terminated"));
            }
            let branch_value = pop_stack(&mut branch_stack, "case branch value")?;
            self.set_terminator(
                end_block,
                MirTerminator::Jump {
                    target: merge_block,
                    args: vec![branch_value.id],
                },
            )?;
        }

        stack.push(merge_value);
        Ok((merge_block, stack))
    }

    fn lower_short_circuit(
        &mut self,
        block_id: u32,
        mut stack: Vec<StackValue>,
        right_ops: &[IrOp],
        op: MirShortCircuitOp,
        offset: usize,
    ) -> Result<(u32, Vec<StackValue>), MirLoweringError> {
        let lhs = pop_stack(&mut stack, "short-circuit lhs")?;
        let rhs_block = self.create_block(Vec::new());
        let short_circuit_block = self.create_block(Vec::new());
        let merge_block = self.create_block(vec![MirType::Dynamic]);
        let merge_value = self.alloc_value(MirType::Dynamic);

        self.set_terminator(
            block_id,
            MirTerminator::ShortCircuit {
                op,
                condition: lhs.id,
                on_evaluate_rhs: rhs_block,
                on_short_circuit: short_circuit_block,
                offset,
            },
        )?;

        let (rhs_end, mut rhs_stack) = self.lower_ops(rhs_block, Vec::new(), right_ops)?;
        if self.block(rhs_end).terminator.is_some() {
            return Err(MirLoweringError::new(
                "short-circuit RHS unexpectedly terminated",
            ));
        }
        let rhs_value = pop_stack(&mut rhs_stack, "short-circuit rhs value")?;
        self.set_terminator(
            rhs_end,
            MirTerminator::Jump {
                target: merge_block,
                args: vec![rhs_value.id],
            },
        )?;

        self.set_terminator(
            short_circuit_block,
            MirTerminator::Jump {
                target: merge_block,
                args: vec![lhs.id],
            },
        )?;

        stack.push(merge_value);
        Ok((merge_block, stack))
    }

    pub(super) fn alloc_value(&mut self, value_type: MirType) -> StackValue {
        let id = self.next_value;
        self.next_value += 1;
        StackValue { id, value_type }
    }

    pub(super) fn create_block(&mut self, arg_types: Vec<MirType>) -> u32 {
        let id = self.blocks.len() as u32;
        let args = arg_types
            .into_iter()
            .enumerate()
            .map(|(index, value_type)| MirTypedName {
                name: format!("b{id}_arg{index}"),
                value_type,
            })
            .collect::<Vec<_>>();

        self.blocks.push(BlockBuilder {
            id,
            args,
            instructions: Vec::new(),
            terminator: None,
        });
        id
    }

    pub(super) fn block(&self, id: u32) -> &BlockBuilder {
        &self.blocks[id as usize]
    }

    pub(super) fn block_mut(&mut self, id: u32) -> &mut BlockBuilder {
        &mut self.blocks[id as usize]
    }

    fn set_terminator(
        &mut self,
        id: u32,
        terminator: MirTerminator,
    ) -> Result<(), MirLoweringError> {
        let block = self.block_mut(id);
        if block.terminator.is_some() {
            return Err(MirLoweringError::new(format!(
                "block {} already has terminator",
                id
            )));
        }
        block.terminator = Some(terminator);
        Ok(())
    }
}

pub(super) fn pop_stack(
    stack: &mut Vec<StackValue>,
    context: &str,
) -> Result<StackValue, MirLoweringError> {
    stack
        .pop()
        .ok_or_else(|| MirLoweringError::new(format!("stack underflow while lowering {context}")))
}

pub(super) fn pop_n(
    stack: &mut Vec<StackValue>,
    count: usize,
    context: &str,
) -> Result<Vec<StackValue>, MirLoweringError> {
    if stack.len() < count {
        return Err(MirLoweringError::new(format!(
            "stack underflow while lowering {context}"
        )));
    }

    let start = stack.len() - count;
    Ok(stack.drain(start..).collect())
}
