use iced_x86::Register;

pub mod compiler;

use crate::model::function::{FunctionSignature};
use crate::model::typesystem::Type;

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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    MoveExplicitToImplicit(HardwareRegister, HardwareRegisterExplicit),
    AddInt32(HardwareRegister, HardwareRegister),
    SubInt32(HardwareRegister, HardwareRegister),
    AddFloat32(HardwareRegister, HardwareRegister),
    SubFloat32(HardwareRegister, HardwareRegister),
    MoveInt32ToMemory(i32, i32),
    Call(FunctionSignature, Vec<Variable>),
    Return,
    NullReferenceCheck(HardwareRegister),
    ArrayBoundsCheck(HardwareRegister, HardwareRegister),
    NewArray(Type, HardwareRegister),
    LoadElement(Type, HardwareRegister, HardwareRegister),
    StoreElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    LoadArrayLength(HardwareRegister),
    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(JumpCondition, Type, BranchLabel, HardwareRegister, HardwareRegister)
}

#[derive(Debug)]
pub enum Variable {
    Register(HardwareRegister),
    OperandStack,
    Memory(i32)
}

impl Variable {
    pub fn move_to_explicit(&self, destination: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::MoveImplicitToExplicit(destination, *source));
            }
            Variable::OperandStack => {
                instructions.push(InstructionIR::PopOperandExplicit(destination));
            }
            Variable::Memory(offset) => {
                instructions.push(InstructionIR::LoadMemoryExplicit(destination, *offset));
            }
        }
    }

    pub fn move_to_stack(&self, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::PushNormal(*source));
            }
            Variable::OperandStack => {
                instructions.push(InstructionIR::PopOperandExplicit(HardwareRegisterExplicit(Register::RAX)));
                instructions.push(InstructionIR::PushNormalExplicit(HardwareRegisterExplicit(Register::RAX)));
            }
            Variable::Memory(offset) => {
                instructions.push(InstructionIR::LoadMemoryExplicit(HardwareRegisterExplicit(Register::RAX), *offset));
                instructions.push(InstructionIR::PushNormalExplicit(HardwareRegisterExplicit(Register::RAX)));
            }
        }
    }

    pub fn move_from_explicit(&self, source: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(destination) => {
                instructions.push(InstructionIR::MoveExplicitToImplicit(*destination, source));
            }
            Variable::OperandStack => {
                instructions.push(InstructionIR::PushOperandExplicit(source));
            }
            Variable::Memory(offset) => {
                instructions.push(InstructionIR::StoreMemoryExplicit(*offset, source));
            }
        }
    }
}