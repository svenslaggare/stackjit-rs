use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::compiler::ir::BranchLabel;
use crate::mir::{InstructionMIR, InstructionMIRData, RegisterMIR};
use crate::model::typesystem::{TypeId, TypeStorage};
use crate::optimization::null_check_elision::InstructionsRegisterNullStatus;
use crate::mir::compiler::{MIRCompilationResult, InstructionMIRCompiler};
use crate::model::function::{Function, FunctionDeclaration};
use crate::model::instruction::Instruction;
use crate::model::binder::Binder;
use crate::model::verifier::Verifier;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;

pub struct OptimizationResult {
    pub instructions_register_null_status: InstructionsRegisterNullStatus
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VirtualRegisterType {
    Int,
    Float
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VirtualRegister {
    pub number: u32,
    pub register_type: VirtualRegisterType
}

impl VirtualRegister {
    pub fn from(register: &RegisterMIR) -> VirtualRegister {
        match register.value_type {
            TypeId::Float32 => VirtualRegister { number: register.number, register_type: VirtualRegisterType::Float },
            _ => VirtualRegister { number: register.number, register_type: VirtualRegisterType::Int }
        }
    }

}
pub fn create_label_mapping(instructions: &Vec<InstructionMIR>) -> HashMap<BranchLabel, usize> {
    let mut mapping = HashMap::new();

    for (instruction_index, instruction) in instructions.iter().enumerate() {
        if let InstructionMIRData::BranchLabel(label) = &instruction.data {
            mapping.insert(*label, instruction_index);
        }
    }

    mapping
}

pub fn determine_instructions_operand_stack(compilation_result: &MIRCompilationResult) -> Vec<Vec<RegisterMIR>> {
    let mut operand_stack = Vec::new();
    let mut instructions_operands = Vec::new();
    let local_registers = HashSet::<RegisterMIR>::from_iter(compilation_result.local_virtual_registers.iter().cloned());

    let pop_if_not_local = |operand_stack: &mut Vec<RegisterMIR>, register: &RegisterMIR| {
        if !local_registers.contains(register) {
            operand_stack.pop();
        }
    };

    let push_if_not_local = |operand_stack: &mut Vec<RegisterMIR>, register: &RegisterMIR| {
        if !local_registers.contains(register) {
            operand_stack.push(register.clone());
        }
    };

    for instruction in &compilation_result.instructions {
        instructions_operands.push(operand_stack.clone());

        for use_register in instruction.data.use_registers().iter().rev() {
            pop_if_not_local(&mut operand_stack, use_register);
        }

        if let Some(assign_register) = instruction.data.assign_register() {
            push_if_not_local(&mut operand_stack, &assign_register);
        }
    }

    instructions_operands
}

#[test]
fn test_determine_instructions_operand_stack1() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let result = compiler.done();

    let instructions_operand_stack = determine_instructions_operand_stack(&result);
    assert_eq!(result.instructions_operand_stack, instructions_operand_stack);
}

#[test]
fn test_determine_instructions_operand_stack2() {
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
    let result = compiler.done();

    let instructions_operand_stack = determine_instructions_operand_stack(&result);
    assert_eq!(result.instructions_operand_stack, instructions_operand_stack);
}