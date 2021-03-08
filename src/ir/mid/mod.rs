use crate::model::typesystem::Type;
use crate::model::function::FunctionSignature;
use crate::ir::low::{BranchLabel, JumpCondition};

pub mod compiler;
pub mod ir_compiler;

#[derive(Clone, PartialEq, Eq)]
pub struct VirtualRegister {
    pub number: u32,
    pub value_type: Type
}

impl std::fmt::Debug for VirtualRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VirtualRegister(number: {}, type: {})", self.number, self.value_type)
    }
}

impl VirtualRegister {
    pub fn new(number: u32, value_type: Type) -> VirtualRegister {
        VirtualRegister {
            number,
            value_type
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionMIR {
    Marker(usize),
    LoadInt32(VirtualRegister, i32),
    LoadFloat32(VirtualRegister, f32),
    Move(VirtualRegister, VirtualRegister),
    AddInt32(VirtualRegister, VirtualRegister, VirtualRegister),
    SubInt32(VirtualRegister, VirtualRegister, VirtualRegister),
    AddFloat32(VirtualRegister, VirtualRegister, VirtualRegister),
    SubFloat32(VirtualRegister, VirtualRegister, VirtualRegister),
    Return(Option<VirtualRegister>),
    Call(FunctionSignature, Option<VirtualRegister>, Vec<VirtualRegister>),
    LoadArgument(u32, VirtualRegister),
    LoadNull(VirtualRegister),
    NewArray(Type, VirtualRegister, VirtualRegister),
    LoadElement(Type, VirtualRegister, VirtualRegister, VirtualRegister),
    StoreElement(Type, VirtualRegister, VirtualRegister, VirtualRegister),
    LoadArrayLength(VirtualRegister, VirtualRegister),
    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(JumpCondition, Type, BranchLabel, VirtualRegister, VirtualRegister)
}

impl InstructionMIR {
    pub fn name(&self) -> String {
        match self {
            InstructionMIR::Marker(_) => "Marker".to_owned(),
            InstructionMIR::LoadInt32(_, _) => "LoadInt32".to_owned(),
            InstructionMIR::LoadFloat32(_, _) => "LoadFloat32".to_owned(),
            InstructionMIR::Move(_, _) => "Move".to_owned(),
            InstructionMIR::AddInt32(_, _, _) => "AddInt32".to_owned(),
            InstructionMIR::SubInt32(_, _, _) => "SubInt32".to_owned(),
            InstructionMIR::AddFloat32(_, _, _) => "AddFloat32".to_owned(),
            InstructionMIR::SubFloat32(_, _, _) => "SubFloat32".to_owned(),
            InstructionMIR::Return(_) => "Return".to_owned(),
            InstructionMIR::Call(_, _, _) => "Call".to_owned(),
            InstructionMIR::LoadArgument(_, _) => "LoadArgument".to_owned(),
            InstructionMIR::LoadNull(_) => "LoadNull".to_owned(),
            InstructionMIR::NewArray(_, _, _) => "NewArray".to_owned(),
            InstructionMIR::LoadElement(_, _, _, _) => "LoadElement".to_owned(),
            InstructionMIR::StoreElement(_, _, _, _) => "StoreElement".to_owned(),
            InstructionMIR::LoadArrayLength(_, _) => "LoadArrayLength".to_owned(),
            InstructionMIR::BranchLabel(_) => "BranchLabel".to_owned(),
            InstructionMIR::Branch(_) => "Branch".to_owned(),
            InstructionMIR::BranchCondition(_, _, _, _, _) => "BranchCondition".to_owned()
        }
    }

    pub fn is_marker(&self) -> bool {
        match self {
            InstructionMIR::Marker(_) => true,
            _ => false
        }
    }
}