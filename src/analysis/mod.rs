use crate::analysis::null_check_elision::InstructionsRegisterNullStatus;
use crate::ir::mid::VirtualRegister;
use crate::model::typesystem::Type;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;
pub mod null_check_elision;

pub struct AnalysisResult {
    pub instructions_register_null_status: InstructionsRegisterNullStatus
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VirtualHardwareRegisterType {
    Int,
    Float
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VirtualHardwareRegister {
    pub number: u32,
    pub register_type: VirtualHardwareRegisterType
}

impl VirtualHardwareRegister {
    pub fn from(register: &VirtualRegister) -> VirtualHardwareRegister {
        match register.value_type {
            Type::Float32 => VirtualHardwareRegister { number: register.number, register_type: VirtualHardwareRegisterType::Float },
            _ => VirtualHardwareRegister { number: register.number, register_type: VirtualHardwareRegisterType::Int }
        }
    }
}