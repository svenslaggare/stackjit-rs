use std::iter::FromIterator;

pub mod compiler;
pub mod branches;

use crate::analysis::VirtualRegister;
use crate::compiler::ir::{BranchLabel, Condition};
use crate::model::function::FunctionSignature;
use crate::model::typesystem::TypeId;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegisterMIR {
    pub number: u32,
    pub value_type: TypeId
}

impl std::fmt::Debug for RegisterMIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegisterMIR(number: {}, type: {})", self.number, self.value_type)
    }
}

impl RegisterMIR {
    pub fn new(number: u32, value_type: TypeId) -> RegisterMIR {
        RegisterMIR {
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

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionMIRData {
    LoadInt32(RegisterMIR, i32),
    LoadFloat32(RegisterMIR, f32),
    LoadBool(RegisterMIR, bool),
    Move(RegisterMIR, RegisterMIR),
    AddInt32(RegisterMIR, RegisterMIR, RegisterMIR),
    AddInt32Constant(RegisterMIR, RegisterMIR, i32),
    SubInt32(RegisterMIR, RegisterMIR, RegisterMIR),
    SubInt32Constant(RegisterMIR, RegisterMIR, i32),
    MultiplyInt32(RegisterMIR, RegisterMIR, RegisterMIR),
    AddFloat32(RegisterMIR, RegisterMIR, RegisterMIR),
    SubFloat32(RegisterMIR, RegisterMIR, RegisterMIR),
    MultiplyFloat32(RegisterMIR, RegisterMIR, RegisterMIR),
    DivideFloat32(RegisterMIR, RegisterMIR, RegisterMIR),
    AndBool(RegisterMIR, RegisterMIR, RegisterMIR),
    OrBool(RegisterMIR, RegisterMIR, RegisterMIR),
    Return(Option<RegisterMIR>),
    Call(FunctionSignature, Option<RegisterMIR>, Vec<RegisterMIR>),
    LoadArgument(u32, RegisterMIR),
    LoadNull(RegisterMIR),
    NewArray(TypeId, RegisterMIR, RegisterMIR),
    LoadElement(TypeId, RegisterMIR, RegisterMIR, RegisterMIR),
    StoreElement(TypeId, RegisterMIR, RegisterMIR, RegisterMIR),
    LoadArrayLength(RegisterMIR, RegisterMIR),
    NewObject(TypeId, RegisterMIR),
    LoadField(TypeId, String, RegisterMIR, RegisterMIR),
    StoreField(TypeId, String, RegisterMIR, RegisterMIR),
    GarbageCollect,
    PrintStackFrame,
    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(Condition, TypeId, BranchLabel, RegisterMIR, RegisterMIR),
    Compare(Condition, TypeId, RegisterMIR, RegisterMIR, RegisterMIR)
}

impl InstructionMIRData {
    pub fn name(&self) -> String {
        match self {
            InstructionMIRData::LoadInt32(_, _) => "LoadInt32".to_owned(),
            InstructionMIRData::LoadFloat32(_, _) => "LoadFloat32".to_owned(),
            InstructionMIRData::LoadBool(_, _) => "LoadBool".to_owned(),
            InstructionMIRData::Move(_, _) => "Move".to_owned(),
            InstructionMIRData::AddInt32(_, _, _) => "AddInt32".to_owned(),
            InstructionMIRData::AddInt32Constant(_, _, _) => "AddInt32Constant".to_owned(),
            InstructionMIRData::SubInt32(_, _, _) => "SubInt32".to_owned(),
            InstructionMIRData::SubInt32Constant(_, _, _) => "AddInt32Constant".to_owned(),
            InstructionMIRData::MultiplyInt32(_, _, _) => "MultiplyInt32".to_owned(),
            InstructionMIRData::AddFloat32(_, _, _) => "AddFloat32".to_owned(),
            InstructionMIRData::SubFloat32(_, _, _) => "SubFloat32".to_owned(),
            InstructionMIRData::MultiplyFloat32(_, _, _) => "MultiplyFloat32".to_owned(),
            InstructionMIRData::DivideFloat32(_, _, _) => "DivideFloat32".to_owned(),
            InstructionMIRData::AndBool(_, _, _) => "AndBool".to_owned(),
            InstructionMIRData::OrBool(_, _, _) => "OrBool".to_owned(),
            InstructionMIRData::Return(_) => "Return".to_owned(),
            InstructionMIRData::Call(_, _, _) => "Call".to_owned(),
            InstructionMIRData::LoadArgument(_, _) => "LoadArgument".to_owned(),
            InstructionMIRData::LoadNull(_) => "LoadNull".to_owned(),
            InstructionMIRData::NewArray(_, _, _) => "NewArray".to_owned(),
            InstructionMIRData::LoadElement(_, _, _, _) => "LoadElement".to_owned(),
            InstructionMIRData::StoreElement(_, _, _, _) => "StoreElement".to_owned(),
            InstructionMIRData::LoadArrayLength(_, _) => "LoadArrayLength".to_owned(),
            InstructionMIRData::NewObject(_, _) => "NewObject".to_owned(),
            InstructionMIRData::GarbageCollect => "GarbageCollect".to_owned(),
            InstructionMIRData::PrintStackFrame => "PrintStackFrame".to_owned(),
            InstructionMIRData::LoadField(_, _, _, _) => "LoadField".to_owned(),
            InstructionMIRData::StoreField(_, _, _, _) => "StoreField".to_owned(),
            InstructionMIRData::BranchLabel(_) => "BranchLabel".to_owned(),
            InstructionMIRData::Branch(_) => "Branch".to_owned(),
            InstructionMIRData::BranchCondition(_, _, _, _, _) => "BranchCondition".to_owned(),
            InstructionMIRData::Compare(_, _, _, _, _) => "Compare".to_owned()
        }
    }

    pub fn assign_register(&self) -> Option<RegisterMIR> {
        match self {
            InstructionMIRData::LoadInt32(register, _) => Some(register.clone()),
            InstructionMIRData::LoadFloat32(register, _) => Some(register.clone()),
            InstructionMIRData::LoadBool(register, _) => Some(register.clone()),
            InstructionMIRData::Move(register, _) => Some(register.clone()),
            InstructionMIRData::AddInt32(register, _, _) => Some(register.clone()),
            InstructionMIRData::AddInt32Constant(register, _, _) => Some(register.clone()),
            InstructionMIRData::SubInt32(register, _, _) => Some(register.clone()),
            InstructionMIRData::SubInt32Constant(register, _, _) => Some(register.clone()),
            InstructionMIRData::MultiplyInt32(register, _, _) => Some(register.clone()),
            InstructionMIRData::AddFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::SubFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::MultiplyFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::DivideFloat32(register, _, _) => Some(register.clone()),
            InstructionMIRData::AndBool(register, _, _) => Some(register.clone()),
            InstructionMIRData::OrBool(register, _, _) => Some(register.clone()),
            InstructionMIRData::Return(_) => None,
            InstructionMIRData::Call(_, register, _) => register.clone(),
            InstructionMIRData::LoadArgument(_, register) => Some(register.clone()),
            InstructionMIRData::LoadNull(register) => Some(register.clone()),
            InstructionMIRData::NewArray(_, register, _) => Some(register.clone()),
            InstructionMIRData::LoadElement(_, register, _, _) => Some(register.clone()),
            InstructionMIRData::NewObject(_, register) => Some(register.clone()),
            InstructionMIRData::GarbageCollect => None,
            InstructionMIRData::PrintStackFrame => None,
            InstructionMIRData::LoadField(_, _, register, _) => Some(register.clone()),
            InstructionMIRData::StoreField(_, _, _, _) => None,
            InstructionMIRData::StoreElement(_, _, _, _) => None,
            InstructionMIRData::LoadArrayLength(_, register) => Some(register.clone()),
            InstructionMIRData::BranchLabel(_) => None,
            InstructionMIRData::Branch(_) => None,
            InstructionMIRData::BranchCondition(_, _, _, _, _) => None,
            InstructionMIRData::Compare(_, _, destination, _, _) => Some(destination.clone())
        }
    }

    pub fn assign_register_mut(&mut self) -> Option<&mut RegisterMIR> {
        match self {
            InstructionMIRData::LoadInt32(register, _) => Some(register),
            InstructionMIRData::LoadFloat32(register, _) => Some(register),
            InstructionMIRData::LoadBool(register, _) => Some(register),
            InstructionMIRData::Move(register, _) => Some(register),
            InstructionMIRData::AddInt32(register, _, _) => Some(register),
            InstructionMIRData::AddInt32Constant(register, _, _) => Some(register),
            InstructionMIRData::SubInt32(register, _, _) => Some(register),
            InstructionMIRData::SubInt32Constant(register, _, _) => Some(register),
            InstructionMIRData::MultiplyInt32(register, _, _) => Some(register),
            InstructionMIRData::AddFloat32(register, _, _) => Some(register),
            InstructionMIRData::SubFloat32(register, _, _) => Some(register),
            InstructionMIRData::MultiplyFloat32(register, _, _) => Some(register),
            InstructionMIRData::DivideFloat32(register, _, _) => Some(register),
            InstructionMIRData::AndBool(register, _, _) => Some(register),
            InstructionMIRData::OrBool(register, _, _) => Some(register),
            InstructionMIRData::Return(_) => None,
            InstructionMIRData::Call(_, register, _) => register.as_mut(),
            InstructionMIRData::LoadArgument(_, register) => Some(register),
            InstructionMIRData::LoadNull(register) => Some(register),
            InstructionMIRData::NewArray(_, register, _) => Some(register),
            InstructionMIRData::LoadElement(_, register, _, _) => Some(register),
            InstructionMIRData::NewObject(_, register) => Some(register),
            InstructionMIRData::GarbageCollect => None,
            InstructionMIRData::PrintStackFrame => None,
            InstructionMIRData::LoadField(_, _, register, _) => Some(register),
            InstructionMIRData::StoreField(_, _, _, _) => None,
            InstructionMIRData::StoreElement(_, _, _, _) => None,
            InstructionMIRData::LoadArrayLength(_, register) => Some(register),
            InstructionMIRData::BranchLabel(_) => None,
            InstructionMIRData::Branch(_) => None,
            InstructionMIRData::BranchCondition(_, _, _, _, _) => None,
            InstructionMIRData::Compare(_, _, destination, _, _) => Some(destination)
        }
    }

    pub fn assign_virtual_register(&self) -> Option<VirtualRegister> {
        self.assign_register().map(|register| VirtualRegister::from(&register))
    }

    pub fn use_registers(&self) -> Vec<RegisterMIR> {
        match self {
            InstructionMIRData::LoadInt32(_, _) => Vec::new(),
            InstructionMIRData::LoadFloat32(_, _) => Vec::new(),
            InstructionMIRData::LoadBool(_, _) => Vec::new(),
            InstructionMIRData::Move(_, op) => vec![op.clone()],
            InstructionMIRData::AddInt32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::AddInt32Constant(_, op1, _) => vec![op1.clone()],
            InstructionMIRData::SubInt32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::SubInt32Constant(_, op1, _) => vec![op1.clone()],
            InstructionMIRData::MultiplyInt32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::AddFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::SubFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::MultiplyFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::DivideFloat32(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::AndBool(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::OrBool(_, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::Return(register) => Vec::from_iter(register.iter().cloned()),
            InstructionMIRData::Call(_, _, arguments) => arguments.clone(),
            InstructionMIRData::LoadArgument(_, _) => Vec::new(),
            InstructionMIRData::LoadNull(_) => Vec::new(),
            InstructionMIRData::NewArray(_, _, op) => vec![op.clone()],
            InstructionMIRData::LoadElement(_, _, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::StoreElement(_, op1, op2, op3) => vec![op1.clone(), op2.clone(), op3.clone()],
            InstructionMIRData::LoadArrayLength(_, _) => Vec::new(),
            InstructionMIRData::NewObject(_, _) => Vec::new(),
            InstructionMIRData::LoadField(_, _, _, op) => vec![op.clone()],
            InstructionMIRData::StoreField(_, _, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::GarbageCollect => Vec::new(),
            InstructionMIRData::PrintStackFrame => Vec::new(),
            InstructionMIRData::BranchLabel(_) => Vec::new(),
            InstructionMIRData::Branch(_) => Vec::new(),
            InstructionMIRData::BranchCondition(_, _, _, op1, op2) => vec![op1.clone(), op2.clone()],
            InstructionMIRData::Compare(_, _, _, op1, op2) => vec![op1.clone(), op2.clone()]
        }
    }

    pub fn use_registers_mut(&mut self) -> Vec<&mut RegisterMIR> {
        match self {
            InstructionMIRData::LoadInt32(_, _) => Vec::new(),
            InstructionMIRData::LoadFloat32(_, _) => Vec::new(),
            InstructionMIRData::LoadBool(_, _) => Vec::new(),
            InstructionMIRData::Move(_, op) => vec![op],
            InstructionMIRData::AddInt32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::AddInt32Constant(_, op1, _) => vec![op1],
            InstructionMIRData::SubInt32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::SubInt32Constant(_, op1, _) => vec![op1],
            InstructionMIRData::MultiplyInt32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::AddFloat32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::SubFloat32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::MultiplyFloat32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::DivideFloat32(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::AndBool(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::OrBool(_, op1, op2) => vec![op1, op2],
            InstructionMIRData::Return(register) => register.as_mut().map(|r| vec![r]).unwrap_or_else(|| Vec::new()),
            InstructionMIRData::Call(_, _, arguments) => arguments.iter_mut().map(|r| r).collect(),
            InstructionMIRData::LoadArgument(_, _) => Vec::new(),
            InstructionMIRData::LoadNull(_) => Vec::new(),
            InstructionMIRData::NewArray(_, _, op) => vec![op],
            InstructionMIRData::LoadElement(_, _, op1, op2) => vec![op1, op2],
            InstructionMIRData::StoreElement(_, op1, op2, op3) => vec![op1, op2, op3],
            InstructionMIRData::LoadArrayLength(_, _) => Vec::new(),
            InstructionMIRData::NewObject(_, _) => Vec::new(),
            InstructionMIRData::LoadField(_, _, _, op) => vec![op],
            InstructionMIRData::StoreField(_, _, op1, op2) => vec![op1, op2],
            InstructionMIRData::GarbageCollect => Vec::new(),
            InstructionMIRData::PrintStackFrame => Vec::new(),
            InstructionMIRData::BranchLabel(_) => Vec::new(),
            InstructionMIRData::Branch(_) => Vec::new(),
            InstructionMIRData::BranchCondition(_, _, _, op1, op2) => vec![op1, op2],
            InstructionMIRData::Compare(_, _, _, op1, op2) => vec![op1, op2]
        }
    }

    pub fn use_virtual_registers(&self) -> Vec<VirtualRegister> {
        self.use_registers().iter().map(|register| VirtualRegister::from(register)).collect()
    }
}
