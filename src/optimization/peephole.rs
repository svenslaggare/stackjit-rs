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

pub fn remove_load_local(compilation_result: &mut MIRCompilationResult, basic_blocks: &mut Vec<BasicBlock>) {
    let local_registers = HashSet::<RegisterMIR>::from_iter(compilation_result.local_virtual_registers.iter().cloned());
    for block in basic_blocks.iter_mut() {
        remove_load_local_for_block(compilation_result, &local_registers, block);
    }

    let valid_instructions = HashSet::<usize>::from_iter(BasicBlock::linearize(basic_blocks).into_iter());

    let mut index = 0;
    compilation_result.instructions.retain(|instruction| {
        let keep = valid_instructions.contains(&index);
        index += 1;
        keep
    });
}

fn remove_load_local_for_block(compilation_result: &mut MIRCompilationResult,
                               local_registers: &HashSet<RegisterMIR>,
                               basic_block: &mut BasicBlock) {
    let mut local_load_target = HashMap::new();
    let mut instructions_to_remove = HashSet::new();

    for &instruction_index in &basic_block.instructions {
        let instruction = &mut compilation_result.instructions[instruction_index];
        match &mut instruction.data {
            InstructionMIRData::Move(destination, source) if local_registers.contains(source) => {
                local_load_target.insert(destination.clone(), (source.clone(), instruction_index));
            }
            InstructionMIRData::Move(destination, _) if local_registers.contains(destination) => {
                local_load_target.remove(destination);
            }
            InstructionMIRData::AddInt32(_, op1, op2)
            | InstructionMIRData::SubInt32(_, op1, op2)
            | InstructionMIRData::AddFloat32(_, op1, op2)
            | InstructionMIRData::SubFloat32(_, op1, op2) => {
                if let Some((op1_new, load_instruction_index)) = local_load_target.remove(op1) {
                    *op1 = op1_new;
                    instructions_to_remove.insert(load_instruction_index);
                }

                if let Some((op2_new, load_instruction_index)) = local_load_target.remove(op2) {
                    *op2 = op2_new;
                    instructions_to_remove.insert(load_instruction_index);
                }
            }
            _ => {}
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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(2, compilation_result.instructions.len());
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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

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
    remove_load_local(&mut compilation_result, &mut basic_blocks);

    println!();
    println!("After optimization:");
    for instruction in &compilation_result.instructions {
        println!("{:?}", instruction);
    }

    assert_eq!(9, compilation_result.instructions.len());
}