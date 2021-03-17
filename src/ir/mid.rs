use std::iter::FromIterator;

use crate::ir::{BranchLabel, Condition};
use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;
use crate::analysis::VirtualHardwareRegister;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
pub struct InstructionMIR {
    pub index: usize,
    pub data: InstructionMIRData
}

impl InstructionMIR {
    pub fn new(index: usize, data: InstructionMIRData) -> InstructionMIR {
        InstructionMIR {
            index,
            data
        }
    }

    pub fn name(&self) -> String {
        self.data.name()
    }
}

impl InstructionMIRData {
    pub fn name(&self) -> String {
        match self {
            InstructionMIRData::LoadInt32(_, _) => "LoadInt32".to_owned(),
            InstructionMIRData::LoadFloat32(_, _) => "LoadFloat32".to_owned(),
            InstructionMIRData::Move(_, _) => "Move".to_owned(),
            InstructionMIRData::AddInt32(_, _, _) => "AddInt32".to_owned(),
            InstructionMIRData::SubInt32(_, _, _) => "SubInt32".to_owned(),
            InstructionMIRData::AddFloat32(_, _, _) => "AddFloat32".to_owned(),
            InstructionMIRData::SubFloat32(_, _, _) => "SubFloat32".to_owned(),
            InstructionMIRData::Return(_) => "Return".to_owned(),
            InstructionMIRData::Call(_, _, _) => "Call".to_owned(),
            InstructionMIRData::LoadArgument(_, _) => "LoadArgument".to_owned(),
            InstructionMIRData::LoadNull(_) => "LoadNull".to_owned(),
            InstructionMIRData::NewArray(_, _, _) => "NewArray".to_owned(),
            InstructionMIRData::LoadElement(_, _, _, _) => "LoadElement".to_owned(),
            InstructionMIRData::StoreElement(_, _, _, _) => "StoreElement".to_owned(),
            InstructionMIRData::LoadArrayLength(_, _) => "LoadArrayLength".to_owned(),
            InstructionMIRData::BranchLabel(_) => "BranchLabel".to_owned(),
            InstructionMIRData::Branch(_) => "Branch".to_owned(),
            InstructionMIRData::BranchCondition(_, _, _, _, _) => "BranchCondition".to_owned()
        }
    }

    pub fn assign_register(&self) -> Option<VirtualRegister> {
        match self {
            InstructionMIRData::LoadInt32(register, _) => Some(register.clone()),
            InstructionMIRData::LoadFloat32(register, _) => Some(register.clone()),
            InstructionMIRData::Move(register, _) => Some(register.clone()),
            InstructionMIRData::AddInt32(register, _, _) => Some(register.clone()),
            InstructionMIRData::SubInt32(register, _, _) => Some(register.clone()),
            InstructionMIRData::AddFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::SubFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::Return(_) => None,
            InstructionMIRData::Call(_, register, _) => register.clone(),
            InstructionMIRData::LoadArgument(_, register) => Some(register.clone()),
            InstructionMIRData::LoadNull(register) => Some(register.clone()),
            InstructionMIRData::NewArray(_, register, _) => Some(register.clone()),
            InstructionMIRData::LoadElement(_, register, _, _) => Some(register.clone()),
            InstructionMIRData::StoreElement(_, _, _, _) => None,
            InstructionMIRData::LoadArrayLength(_, register) => Some(register.clone()),
            InstructionMIRData::BranchLabel(_) => None,
            InstructionMIRData::Branch(_) => None,
            InstructionMIRData::BranchCondition(_, _, _, _, _) => None
        }
    }

    pub fn assign_hardware_register(&self) -> Option<VirtualHardwareRegister> {
        self.assign_register().map(|register| VirtualHardwareRegister::from(&register))
    }

    pub fn use_registers(&self) -> Vec<VirtualRegister> {
        match self {
            InstructionMIRData::LoadInt32(_, _) => Vec::new(),
            InstructionMIRData::LoadFloat32(_, _) => Vec::new(),
            InstructionMIRData::Move(_, op) => vec![op.clone()],
            InstructionMIRData::AddInt32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::SubInt32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::AddFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::SubFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::Return(register) => Vec::from_iter(register.iter().cloned()),
            InstructionMIRData::Call(_, _, arguments) => arguments.clone(),
            InstructionMIRData::LoadArgument(_, _) => Vec::new(),
            InstructionMIRData::LoadNull(_) => Vec::new(),
            InstructionMIRData::NewArray(_, _, op) => vec![op.clone()],
            InstructionMIRData::LoadElement(_, _, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::StoreElement(_, op1, op2, op3) => vec![op1.clone(), op2.clone(), op3.clone()],
            InstructionMIRData::LoadArrayLength(_, _) => Vec::new(),
            InstructionMIRData::BranchLabel(_) => Vec::new(),
            InstructionMIRData::Branch(_) => Vec::new(),
            InstructionMIRData::BranchCondition(_, _, _, op1, op2) => vec![op1.clone(), op2.clone()]
        }
    }

    pub fn use_hardware_registers(&self) -> Vec<VirtualHardwareRegister> {
        self.use_registers().iter().map(|register| VirtualHardwareRegister::from(register)).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionMIRData {
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
    BranchCondition(Condition, Type, BranchLabel, VirtualRegister, VirtualRegister)
}
