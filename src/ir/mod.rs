use iced_x86::Register;

pub mod mid;
pub mod compiler;
pub mod ir_compiler;
pub mod optimized_ir_compiler;
pub mod branches;

use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareRegister {
    Int(u32),
    IntSpill,
    Float(u32),
    FloatSpill
}

impl std::fmt::Debug for HardwareRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareRegister::Int(value) => {
                write!(f, "HardwareRegister::Int({})", value)
            }
            HardwareRegister::IntSpill => {
                write!(f, "HardwareRegister::IntSpill")
            }
            HardwareRegister::Float(value) => {
                write!(f, "HardwareRegister::Float({})", value)
            }
            HardwareRegister::FloatSpill => {
                write!(f, "HardwareRegister::FloatSpill")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HardwareRegisterExplicit(pub iced_x86::Register);

pub type BranchLabel = u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
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
    LoadZeroToRegister(HardwareRegister),
    AddToStackPointer(i32),
    SubFromStackPointer(i32),
    Push(HardwareRegister),
    Pop(HardwareRegister),
    PushExplicit(HardwareRegisterExplicit),
    PopExplicit(HardwareRegisterExplicit),

    LoadFrameMemory(HardwareRegister, i32),
    StoreFrameMemory(i32, HardwareRegister),
    LoadFrameMemoryExplicit(HardwareRegisterExplicit, i32),
    StoreFrameMemoryExplicit(i32, HardwareRegisterExplicit),
    LoadStackMemory(HardwareRegister, i32),
    StoreStackMemory(i32, HardwareRegister),
    LoadStackMemoryExplicit(HardwareRegisterExplicit, i32),
    StoreStackMemoryExplicit(i32, HardwareRegisterExplicit),

    Move(HardwareRegister, HardwareRegister),
    MoveImplicitToExplicit(HardwareRegisterExplicit, HardwareRegister),
    MoveExplicitToImplicit(HardwareRegister, HardwareRegisterExplicit),

    MoveInt32ToFrameMemory(i32, i32),
    MoveInt32ToRegister(HardwareRegister, i32),

    AddInt32(HardwareRegister, HardwareRegister),
    AddInt32FromFrameMemory(HardwareRegister, i32),
    AddInt32ToFrameMemory(i32, HardwareRegister),
    SubInt32(HardwareRegister, HardwareRegister),
    SubInt32FromFrameMemory(HardwareRegister, i32),
    SubInt32ToFrameMemory(i32, HardwareRegister),

    AddFloat32(HardwareRegister, HardwareRegister),
    SubFloat32(HardwareRegister, HardwareRegister),

    Call(FunctionSignature, Vec<Variable>),
    Return,

    NullReferenceCheck(HardwareRegister),
    ArrayBoundsCheck(HardwareRegister, HardwareRegister),

    NewArray(Type, HardwareRegister),
    LoadElement(Type, HardwareRegister, HardwareRegister),
    StoreElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    LoadArrayLength(HardwareRegister),

    Compare(Type, HardwareRegister, HardwareRegister),
    CompareFromFrameMemory(Type, HardwareRegister, i32),
    CompareToFrameMemory(Type, i32, HardwareRegister),

    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(Condition, bool, BranchLabel)
}

#[derive(Debug)]
pub enum Variable {
    Register(HardwareRegister),
    FrameMemory(i32),
    StackMemory(i32)
}

impl Variable {
    pub fn move_to_explicit(&self, destination: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::MoveImplicitToExplicit(destination, *source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemoryExplicit(destination, *offset));
            }
            Variable::StackMemory(offset) => {
                instructions.push(InstructionIR::LoadStackMemoryExplicit(destination, *offset));
            }
        }
    }

    pub fn move_to_stack(&self, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::Push(*source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemoryExplicit(HardwareRegisterExplicit(Register::RAX), *offset));
                instructions.push(InstructionIR::PushExplicit(HardwareRegisterExplicit(Register::RAX)));
            }
            Variable::StackMemory(offset) => {
                instructions.push(InstructionIR::LoadStackMemoryExplicit(HardwareRegisterExplicit(Register::RAX), *offset));
                instructions.push(InstructionIR::PushExplicit(HardwareRegisterExplicit(Register::RAX)));
            }
        }
    }

    pub fn move_from_explicit(&self, source: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(destination) => {
                instructions.push(InstructionIR::MoveExplicitToImplicit(*destination, source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::StoreFrameMemoryExplicit(*offset, source));
            }
            Variable::StackMemory(offset) => {
                instructions.push(InstructionIR::StoreStackMemoryExplicit(*offset, source));
            }
        }
    }
}
