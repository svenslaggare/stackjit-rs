pub mod compiler;

use crate::model::function::FunctionSignature;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum  VirtualRegister {
    Int(u32),
    Float(u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HardwareRegister(pub iced_x86::Register);

#[derive(Debug)]
pub enum InstructionIR {
    Marker(usize),
    InitializeFunction,
    LoadInt32(i32),
    LoadZeroToRegister(VirtualRegister),
    AddToStackPointer(i32),
    SubFromStackPointer(i32),
    PushOperand(VirtualRegister),
    PopOperand(VirtualRegister),
    PushOperandHardware(HardwareRegister),
    PopOperandHardware(HardwareRegister),
    PushNormalHardware(HardwareRegister),
    PopNormalHardware(HardwareRegister),
    LoadMemory(VirtualRegister, i32),
    StoreMemory(i32, VirtualRegister),
    AddInt32(VirtualRegister, VirtualRegister),
    SubInt32(VirtualRegister, VirtualRegister),
    AddFloat32(VirtualRegister, VirtualRegister),
    SubFloat32(VirtualRegister, VirtualRegister),
    MoveMemoryToHardware(HardwareRegister, i32),
    MoveHardwareToMemory(i32, HardwareRegister),
    MoveInt32ToMemory(i32, i32),
    Call(FunctionSignature),
    Return
}