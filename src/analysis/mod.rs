use std::collections::HashMap;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;
pub mod null_check_elision;

use crate::compiler::ir::BranchLabel;
use crate::analysis::null_check_elision::InstructionsRegisterNullStatus;
use crate::mir::{RegisterMIR, InstructionMIR, InstructionMIRData};
use crate::model::typesystem::Type;

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
            Type::Float32 => VirtualRegister { number: register.number, register_type: VirtualRegisterType::Float },
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