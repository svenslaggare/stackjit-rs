use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareRegister {
    Int(u32),
    IntSpill,
    Float(u32),
    FloatSpill,
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

#[derive(Debug)]
pub enum Variable {
    Register(HardwareRegister),
    FrameMemory(i32)
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
        }
    }

    pub fn move_to_stack(&self, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::Push(*source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, *offset));
                instructions.push(InstructionIR::Push(HardwareRegister::IntSpill));
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
    Marker(usize, usize),
    InitializeFunction,
    LoadZeroToRegister(HardwareRegister),
    AddToStackPointer(i32),
    SubFromStackPointer(i32),

    Push(HardwareRegister),
    Pop(HardwareRegister),
    PushExplicit(HardwareRegisterExplicit),
    PopExplicit(HardwareRegisterExplicit),
    PopEmpty,
    PushInt32(i32),

    LoadFrameMemory(HardwareRegister, i32),
    StoreFrameMemory(i32, HardwareRegister),
    LoadFrameMemoryExplicit(HardwareRegisterExplicit, i32),
    StoreFrameMemoryExplicit(i32, HardwareRegisterExplicit),

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
    AddFloat32FromFrameMemory(HardwareRegister, i32),
    SubFloat32(HardwareRegister, HardwareRegister),
    SubFloat32FromFrameMemory(HardwareRegister, i32),

    Call(FunctionSignature, Vec<Variable>, usize),
    Return,

    NullReferenceCheck(HardwareRegister),
    ArrayBoundsCheck(HardwareRegister, HardwareRegister),

    NewArray(Type, HardwareRegister, usize),
    LoadElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    StoreElement(Type, HardwareRegister, HardwareRegister, HardwareRegister),
    LoadArrayLength(HardwareRegister, HardwareRegister),

    NewObject(Type),
    LoadField(Type, usize, HardwareRegister, HardwareRegister),
    StoreField(Type, usize, HardwareRegister, HardwareRegister),

    Compare(Type, HardwareRegister, HardwareRegister),
    CompareFromFrameMemory(Type, HardwareRegister, i32),
    CompareToFrameMemory(Type, i32, HardwareRegister),

    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(Condition, bool, BranchLabel),

    PrintStackFrame(usize),
    GarbageCollect(usize)
}
