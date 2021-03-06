use crate::model::typesystem::Type;
use crate::model::function::FunctionSignature;

pub mod compiler;
pub mod mir_to_ir;

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

#[derive(Debug)]
pub enum InstructionMIR {
    Marker(usize),
    LoadInt32(VirtualRegister, i32),
    Move(VirtualRegister, VirtualRegister),
    AddInt32(VirtualRegister, VirtualRegister, VirtualRegister),
    SubInt32(VirtualRegister, VirtualRegister, VirtualRegister),
    AddFloat32(VirtualRegister, VirtualRegister, VirtualRegister),
    SubFloat32(VirtualRegister, VirtualRegister, VirtualRegister),
    Return(Option<VirtualRegister>),
    Call(FunctionSignature, Option<VirtualRegister>, Vec<VirtualRegister>)
}
