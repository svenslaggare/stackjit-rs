use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;

pub mod compiler;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardwareRegister {
    Int(u32),
    Float(u32)
}

impl std::fmt::Debug for HardwareRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareRegister::Int(value) => {
                write!(f, "HardwareRegister::Int({})", value)
            }
            HardwareRegister::Float(value) => {
                write!(f, "HardwareRegister::Float({})", value)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HardwareRegisterExplicit(pub iced_x86::Register);

pub type BranchLabel = u32;

#[derive(Debug)]
pub enum JumpCondition {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual
}

#[derive(Debug)]
pub enum CallArgumentSource {
    Register(HardwareRegister),
    OperandStack,
    Memory(i32)
}

#[derive(Debug)]
pub enum InstructionIR {
    Marker(usize),
    InitializeFunction,
    LoadInt32(i32),
    LoadZeroToRegister(HardwareRegister),
    AddToStackPointer(i32),
    SubFromStackPointer(i32),
    PushOperand(HardwareRegister),
    PopOperand(HardwareRegister),
    PushNormal(HardwareRegister),
    PopNormal(HardwareRegister),
    PushOperandExplicit(HardwareRegisterExplicit),
    PopOperandExplicit(HardwareRegisterExplicit),
    PushNormalExplicit(HardwareRegisterExplicit),
    PopNormalExplicit(HardwareRegisterExplicit),
    LoadMemory(HardwareRegister, i32),
    StoreMemory(i32, HardwareRegister),
    LoadMemoryExplicit(HardwareRegisterExplicit, i32),
    StoreMemoryExplicit(i32, HardwareRegisterExplicit),
    MoveImplicitToExplicit(HardwareRegisterExplicit, HardwareRegister),
    AddInt32(HardwareRegister, HardwareRegister),
    SubInt32(HardwareRegister, HardwareRegister),
    AddFloat32(HardwareRegister, HardwareRegister),
    SubFloat32(HardwareRegister, HardwareRegister),
    MoveInt32ToMemory(i32, i32),
    Call(FunctionSignature, Vec<CallArgumentSource>),
    Return,
    NullReferenceCheck(HardwareRegister),
    ArrayBoundsCheck(HardwareRegister, HardwareRegister),
    NewArray(Type),
    LoadElement(Type, HardwareRegister, HardwareRegister),
    StoreElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    LoadArrayLength(HardwareRegister),
    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(JumpCondition, Type, BranchLabel, HardwareRegister, HardwareRegister)
}
