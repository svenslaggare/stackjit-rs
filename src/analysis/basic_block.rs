use std::collections::BTreeSet;
use std::iter::FromIterator;

use crate::engine::binder::Binder;
use crate::ir::branches;
use crate::ir::mid::InstructionMIR;
use crate::ir::compiler::InstructionMIRCompiler;
use crate::ir::mid::InstructionMIRData;
use crate::model::function::{Function, FunctionDefinition};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::model::verifier::Verifier;

pub struct BasicBlock {
    pub start_offset: usize,
    pub instructions: Vec<usize>
}

impl BasicBlock {
    pub fn first(&self) -> usize {
        *self.instructions.first().unwrap()
    }

    pub fn last(&self) -> usize {
        *self.instructions.last().unwrap()
    }

    pub fn create_blocks(instructions: &Vec<InstructionMIR>) -> Vec<BasicBlock> {
        let mut blocks = Vec::new();
        let leaders = BasicBlock::find_leaders(instructions);

        for (leader_index, &leader) in leaders.iter().enumerate() {
            let mut block_instructions = Vec::new();

            if leader_index + 1 < leaders.len() {
                for instruction_index in leader..leaders[leader_index + 1] {
                    block_instructions.push(instruction_index);
                }
            } else {
                for instruction_index in leader..instructions.len() {
                    block_instructions.push(instruction_index);
                }
            }

            blocks.push(BasicBlock {
                start_offset: leader,
                instructions: block_instructions
            });
        }

        blocks
    }

    fn find_leaders(instructions: &Vec<InstructionMIR>) -> Vec<usize> {
        let branch_label_mapping = branches::create_label_mapping(instructions);

        // A leader is the start of a basic block
        let mut leaders = BTreeSet::new();
        let mut prev_is_branch = false;
        for (instruction_index, instruction) in instructions.iter().enumerate() {
            if instruction_index == 0 {
                leaders.insert(instruction_index);
                continue;
            }

            match &instruction.data {
                InstructionMIRData::Branch(label) | InstructionMIRData::BranchCondition(_, _, label, _, _) => {
                    leaders.insert(branch_label_mapping[label]);
                    prev_is_branch = true;
                    continue;
                }
                InstructionMIRData::Return(_) => {
                    prev_is_branch = true;
                    continue;
                }
                _ => {}
            }

            if prev_is_branch {
                leaders.insert(instruction_index);
                prev_is_branch = false;
            }
        }

        Vec::from_iter(leaders.into_iter())
    }

    pub fn linearize(blocks: &Vec<BasicBlock>) -> Vec<usize> {
        let mut instructions = Vec::new();

        for block in blocks {
            instructions.extend(block.instructions.iter().cloned());
        }

        instructions
    }
}

fn get_instructions(instructions: &Vec<InstructionMIR>, indices: &Vec<usize>) -> Vec<InstructionMIR> {
    indices.iter().map(|index| instructions[*index].clone()).collect()
}

#[test]
fn test_no_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::Add,
            Instruction::LoadInt32(3),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let instructions = compiler.done().instructions;

    let blocks = BasicBlock::create_blocks(&instructions);

    assert_eq!(1, blocks.len());
    assert_eq!(instructions.len(), blocks[0].instructions.len());
}

#[test]
fn test_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::BranchNotEqual(6),
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut instructions = compiler.done().instructions;

    let blocks = BasicBlock::create_blocks(&instructions);

    assert_eq!(4, blocks.len());
    assert_eq!(instructions, get_instructions(&instructions, &BasicBlock::linearize(&blocks)));

    let mut linearized_instructions = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        if block_index + 1 < blocks.len() {
            let expected_block_instructions = &instructions[block.start_offset..blocks[block_index + 1].start_offset];
            assert_eq!(&get_instructions(&instructions, &block.instructions)[..], expected_block_instructions);
            linearized_instructions.extend(expected_block_instructions.iter().cloned())
        }
    }

    linearized_instructions.extend(get_instructions(&instructions, &blocks.last().unwrap().instructions));

    assert_eq!(instructions, linearized_instructions);
}
