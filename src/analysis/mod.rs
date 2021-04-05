use std::collections::{HashMap, HashSet};

use crate::compiler::ir::BranchLabel;
use crate::mir::{InstructionMIR, InstructionMIRData, RegisterMIR};
use crate::model::typesystem::TypeId;
use crate::optimization::null_check_elision::InstructionsRegisterNullStatus;
use crate::mir::compiler::MIRCompilationResult;
use std::iter::FromIterator;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;

pub struct AnalysisResult {
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

pub fn determine_instructions_operand_stack(compilation_result: &mut MIRCompilationResult) -> Vec<Vec<RegisterMIR>> {
    let mut operand_stack = Vec::new();
    let mut instructions_operands = Vec::new();
    let local_registers = HashSet::<RegisterMIR>::from_iter(compilation_result.local_virtual_registers.iter().cloned());

    let mut pop_if_not_local = |operand_stack: &mut Vec<RegisterMIR>, register: &RegisterMIR| {
        if !local_registers.contains(register) {
            operand_stack.pop();
        }
    };

    let mut push_if_not_local = |operand_stack: &mut Vec<RegisterMIR>, register: &RegisterMIR| {
        if !local_registers.contains(register) {
            operand_stack.push(register.clone());
        }
    };

    for instruction in &compilation_result.instructions {
        instructions_operands.push(operand_stack.clone());

        match &instruction.data {
            InstructionMIRData::LoadInt32(destination, _) => {
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::LoadFloat32(destination, _) => {
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::Move(destination, source) => {
                pop_if_not_local(&mut operand_stack, source);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::AddInt32(destination, op1, op2)
            | InstructionMIRData::SubInt32(destination, op1, op2)
            | InstructionMIRData::AddFloat32(destination, op1, op2)
            | InstructionMIRData::SubFloat32(destination, op1, op2)=> {
                pop_if_not_local(&mut operand_stack, op2);
                pop_if_not_local(&mut operand_stack, op1);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::Return(source) => {
                if let Some(source) = source {
                    push_if_not_local(&mut operand_stack, source);
                }
            }
            InstructionMIRData::Call(_, destination, arguments) => {
                for argument in arguments {
                    pop_if_not_local(&mut operand_stack, argument);
                }

                if let Some(destination) = destination {
                    push_if_not_local(&mut operand_stack, destination);
                }
            }
            InstructionMIRData::LoadArgument(_, destination) => {
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::LoadNull(destination) => {
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::NewArray(_, destination, size) => {
                pop_if_not_local(&mut operand_stack, size);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::LoadElement(_, destination, array_ref, index) => {
                pop_if_not_local(&mut operand_stack, index);
                pop_if_not_local(&mut operand_stack, array_ref);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::StoreElement(_, array_ref, index, value) => {
                pop_if_not_local(&mut operand_stack, value);
                pop_if_not_local(&mut operand_stack, index);
                pop_if_not_local(&mut operand_stack, array_ref);
            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {
                pop_if_not_local(&mut operand_stack, array_ref);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::NewObject(_, destination) => {
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::LoadField(_, _, destination, class_ref) => {
                pop_if_not_local(&mut operand_stack, class_ref);
                push_if_not_local(&mut operand_stack, destination);
            }
            InstructionMIRData::StoreField(_, _, class_ref, value) => {
                pop_if_not_local(&mut operand_stack, value);
                pop_if_not_local(&mut operand_stack, class_ref);
            }
            InstructionMIRData::GarbageCollect => {}
            InstructionMIRData::BranchLabel(_) => {}
            InstructionMIRData::Branch(_) => {}
            InstructionMIRData::BranchCondition(_, _, _, op1, op2) => {
                pop_if_not_local(&mut operand_stack, op2);
                pop_if_not_local(&mut operand_stack, op1);
            }
        }
    }

    instructions_operands
}