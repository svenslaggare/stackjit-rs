use crate::analysis::null_check_elision::InstructionsRegisterNullStatus;
use crate::ir::mid::RegisterMIR;
use crate::model::typesystem::Type;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;
pub mod null_check_elision;

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