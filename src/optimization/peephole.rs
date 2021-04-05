use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::model::function::{Function, FunctionDeclaration};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{TypeId, TypeStorage};
use crate::model::binder::Binder;
use crate::mir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::analysis::basic_block::BasicBlock;
use crate::model::verifier::Verifier;
use crate::mir::{InstructionMIR, InstructionMIRData, RegisterMIR};
use crate::analysis::determine_instructions_operand_stack;

pub struct PeepholeSettings {
    pub remove_load_local: bool,
    pub remove_store_local: bool
}

impl Default for PeepholeSettings {
    fn default() -> Self {
        PeepholeSettings {
            remove_load_local: true,
            remove_store_local: true
        }
    }
}

impl PeepholeSettings {
    pub fn remove_only_load_local() -> PeepholeSettings {
        PeepholeSettings {
            remove_load_local: true,
            remove_store_local: false
        }
    }

    pub fn remove_only_store_local() -> PeepholeSettings {
        PeepholeSettings {
            remove_load_local: false,
            remove_store_local: true
        }
    }
}

pub fn remove_unnecessary_locals(compilation_result: &mut MIRCompilationResult,
                                 basic_blocks: &mut Vec<BasicBlock>,
                                 settings: &PeepholeSettings) {
    let local_registers = HashSet::<RegisterMIR>::from_iter(compilation_result.local_virtual_registers.iter().cloned());
    for block in basic_blocks.iter_mut() {
        remove_unnecessary_local_for_block(compilation_result, &local_registers, block, settings);
    }

    let valid_instructions = HashSet::<usize>::from_iter(BasicBlock::linearize(basic_blocks).into_iter());

    let mut index = 0;
    compilation_result.instructions.retain(|_| {
        let keep = valid_instructions.contains(&index);
        index += 1;
        keep
    });

    let mut index = 0;
    compilation_result.instructions_operand_stack.retain(|_| {
        let keep = valid_instructions.contains(&index);
        index += 1;
        keep
    });

    compilation_result.instructions_operand_stack = determine_instructions_operand_stack(compilation_result);
}

fn remove_unnecessary_local_for_block(compilation_result: &mut MIRCompilationResult,
                                      local_registers: &HashSet<RegisterMIR>,
                                      basic_block: &mut BasicBlock,
                                      settings: &PeepholeSettings) {
    let mut local_load_target = HashMap::new();
    let mut instructions_to_remove = HashSet::new();
    let mut prev_instruction_assign: Option<(RegisterMIR, usize)> = None;

    for &instruction_index in &basic_block.instructions {
        let instruction = &mut compilation_result.instructions[instruction_index];
        match &mut instruction.data {
            // Remove LoadLocal
            InstructionMIRData::Move(destination, source) if local_registers.contains(source) => {
                if settings.remove_load_local {
                    local_load_target.insert(destination.clone(), (source.clone(), instruction_index));
                }
            }
            InstructionMIRData::Move(destination, _) if local_registers.contains(destination) => {
                local_load_target.remove(destination);
            }
            instruction => {
                for op_register in instruction.use_registers_mut() {
                    if let Some((op_register_new, load_instruction_index)) = local_load_target.remove(op_register) {
                        *op_register = op_register_new;
                        instructions_to_remove.insert(load_instruction_index);
                    }
                }
            }
        }

        for use_register in instruction.data.use_registers() {
            local_load_target.remove(&use_register);
        }

        // Remove StoreLocal
        if settings.remove_store_local {
            let instruction = &compilation_result.instructions[instruction_index];
            if let InstructionMIRData::Move(destination, source) = &instruction.data {
                if local_registers.contains(destination) {
                    if let Some((prev_instruction_assign, prev_instruction_index)) = prev_instruction_assign.as_ref() {
                        if prev_instruction_assign == source {
                            let destination = destination.clone();
                            *compilation_result.instructions[*prev_instruction_index].data.assign_register_mut().unwrap() = destination;
                            instructions_to_remove.insert(instruction_index);
                        }
                    }
                }
            }

            let instruction = &compilation_result.instructions[instruction_index];
            prev_instruction_assign = instruction.data.assign_register().zip(Some(instruction_index));
        }
    }

    basic_block.instructions.retain(|index| !instructions_to_remove.contains(index));
}

#[test]
fn test_combine_load_local1() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(1, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(1, InstructionMIRData::Return(Some(RegisterMIR::new(0, TypeId::Int32)))),
        &compilation_result.instructions[0]
    );
}

#[test]
fn test_combine_load_local2() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(3, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(2, InstructionMIRData::AddInt32(RegisterMIR::new(1, TypeId::Int32), RegisterMIR::new(1, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32))),
        &compilation_result.instructions[1]
    );
}

#[test]
fn test_combine_load_local3() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(2, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(2, InstructionMIRData::AddInt32(RegisterMIR::new(2, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32), RegisterMIR::new(1, TypeId::Int32))),
        &compilation_result.instructions[0]
    );
}

#[test]
fn test_combine_load_local4() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(1),

            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,

            Instruction::LoadLocal(0),
            Instruction::Add,

            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(7, compilation_result.instructions.len());
    assert_eq!(0, compilation_result.instructions[0].index);
    assert_eq!(1, compilation_result.instructions[1].index);
    assert_eq!(2, compilation_result.instructions[2].index);
    assert_eq!(3, compilation_result.instructions[3].index);

    assert_eq!(
        &InstructionMIR::new(6, InstructionMIRData::AddInt32(RegisterMIR::new(2, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32), RegisterMIR::new(1, TypeId::Int32))),
        &compilation_result.instructions[4]
    );

    assert_eq!(
        &InstructionMIR::new(8, InstructionMIRData::AddInt32(RegisterMIR::new(2, TypeId::Int32), RegisterMIR::new(2, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32))),
        &compilation_result.instructions[5]
    );
}

#[test]
fn test_combine_load_local5() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(2, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(2, InstructionMIRData::AddInt32(RegisterMIR::new(1, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32))),
        &compilation_result.instructions[0]
    );
}

#[test]
fn test_combine_load_local6() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(200000),
            Instruction::LoadLocal(0),
            Instruction::BranchGreaterThan(0),

            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_load_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(7, compilation_result.instructions.len());
}

#[test]
fn test_combine_store_local1() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &PeepholeSettings::remove_only_store_local());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(3, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(0, InstructionMIRData::LoadInt32(RegisterMIR::new(0, TypeId::Int32), 4711)),
        &compilation_result.instructions[0]
    );

    assert_eq!(2, compilation_result.instructions[1].index);
}


#[test]
fn test_combine_load_store_local1() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut compilation_result = compiler.done();

    println!("Before optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    let mut basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    remove_unnecessary_locals(&mut compilation_result, &mut basic_blocks, &Default::default());

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(3, compilation_result.instructions.len());
    assert_eq!(
        &InstructionMIR::new(2, InstructionMIRData::AddInt32(RegisterMIR::new(0, TypeId::Int32), RegisterMIR::new(0, TypeId::Int32), RegisterMIR::new(1, TypeId::Int32))),
        &compilation_result.instructions[0]
    );
}