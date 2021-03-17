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
pub enum VirtualHardwareRegister {
    Int(u32),
    Float(u32)
}

impl VirtualHardwareRegister {
    pub fn from(register: &VirtualRegister) -> VirtualHardwareRegister {
        match register.value_type {
            Type::Float32 => VirtualHardwareRegister::Float(register.number),
            _ => VirtualHardwareRegister::Int(register.number)
        }
    }

    pub fn number(&self) -> u32 {
        match self {
            VirtualHardwareRegister::Int(number) => *number,
            VirtualHardwareRegister::Float(number) => *number
        }
    }
}