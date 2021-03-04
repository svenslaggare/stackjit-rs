pub mod compiler;

use iced_x86::Register;

use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardwareRegister {
    Int(u32),
    Float(u32)
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
pub enum InstructionIR {
    Marker(usize),
    InitializeFunction,
    LoadInt32(i32),
    LoadZeroToRegister(HardwareRegister),
    AddToStackPointer(i32),
    SubFromStackPointer(i32),
    PushOperand(HardwareRegister),
    PopOperand(HardwareRegister),
    PushOperandExplicit(HardwareRegisterExplicit),
    PopOperandExplicit(HardwareRegisterExplicit),
    PushNormalExplicit(HardwareRegisterExplicit),
    PopNormalExplicit(HardwareRegisterExplicit),
    LoadMemory(HardwareRegister, i32),
    StoreMemory(i32, HardwareRegister),
    LoadMemoryExplicit(HardwareRegisterExplicit, i32),
    StoreMemoryExplicit(i32, HardwareRegisterExplicit),
    AddInt32(HardwareRegister, HardwareRegister),
    SubInt32(HardwareRegister, HardwareRegister),
    AddFloat32(HardwareRegister, HardwareRegister),
    SubFloat32(HardwareRegister, HardwareRegister),
    MoveInt32ToMemory(i32, i32),
    Call(FunctionSignature),
    Return,
    NullReferenceCheck(HardwareRegister),
    NewArray(Type),
    LoadElement(Type, HardwareRegister, HardwareRegister),
    StoreElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(JumpCondition, Type, BranchLabel, HardwareRegister, HardwareRegister)
}