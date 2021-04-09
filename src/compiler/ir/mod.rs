use crate::model::function::FunctionSignature;
use crate::model::typesystem::TypeId;

pub mod compiler;
pub mod allocated_compiler;
pub mod helpers;

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
    RegisterExplicit(HardwareRegisterExplicit),
    FrameMemory(i32)
}

impl Variable {
    pub fn move_to_register(&self, destination: HardwareRegister, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::Move(destination, *source));
            }
            Variable::RegisterExplicit(source) => {
                instructions.push(InstructionIR::MoveExplicitToImplicit(destination, *source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemory(destination, *offset));
            }
        }
    }

    pub fn move_to_explicit(&self, destination: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::MoveImplicitToExplicit(destination, *source));
            }
            Variable::RegisterExplicit(source) => {
                instructions.push(InstructionIR::MoveExplicit(destination, *source));
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
            Variable::RegisterExplicit(source) => {
                instructions.push(InstructionIR::PushExplicit(*source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, *offset));
                instructions.push(InstructionIR::Push(HardwareRegister::IntSpill));
            }
        }
    }

    pub fn move_to_stack_frame(&self, frame_offset: i32, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(source) => {
                instructions.push(InstructionIR::StoreFrameMemory(frame_offset, *source));
            }
            Variable::RegisterExplicit(source) => {
                instructions.push(InstructionIR::StoreFrameMemoryExplicit(frame_offset, *source));
            }
            Variable::FrameMemory(offset) => {
                instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, *offset));
                instructions.push(InstructionIR::StoreFrameMemory(frame_offset, HardwareRegister::IntSpill));
            }
        }
    }


    pub fn move_from_explicit(&self, source: HardwareRegisterExplicit, instructions: &mut Vec<InstructionIR>) {
        match self {
            Variable::Register(destination) => {
                instructions.push(InstructionIR::MoveExplicitToImplicit(*destination, source));
            }
            Variable::RegisterExplicit(destination) => {
                instructions.push(InstructionIR::MoveExplicit(*destination, source));
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
    MoveExplicit(HardwareRegisterExplicit, HardwareRegisterExplicit),
    MoveImplicitToExplicit(HardwareRegisterExplicit, HardwareRegister),
    MoveExplicitToImplicit(HardwareRegister, HardwareRegisterExplicit),

    MoveInt32ToFrameMemory(i32, i32),
    MoveInt32ToRegister(HardwareRegister, i32),

    AddInt32(HardwareRegister, HardwareRegister),
    AddInt32FromFrameMemory(HardwareRegister, i32),
    AddInt32ToFrameMemory(i32, HardwareRegister),
    AddInt32Constant(HardwareRegister, i32),
    AddInt32ConstantToFrameMemory(i32, i32),

    SubInt32(HardwareRegister, HardwareRegister),
    SubInt32FromFrameMemory(HardwareRegister, i32),
    SubInt32ToFrameMemory(i32, HardwareRegister),
    SubInt32Constant(HardwareRegister, i32),
    SubInt32ConstantToFrameMemory(i32, i32),

    MultiplyInt32(HardwareRegister, HardwareRegister),
    MultiplyInt32FromFrameMemory(HardwareRegister, i32),

    DivideInt32(HardwareRegister, HardwareRegister),
    DivideInt32FromFrameMemory(HardwareRegister, i32),

    AndInt32(HardwareRegister, HardwareRegister),
    AndInt32FromFrameMemory(HardwareRegister, i32),
    AndInt32ToFrameMemory(i32, HardwareRegister),
    AndInt32Constant(HardwareRegister, i32),
    AndInt32ConstantToFrameMemory(i32, i32),

    OrInt32(HardwareRegister, HardwareRegister),
    OrInt32FromFrameMemory(HardwareRegister, i32),
    OrInt32ToFrameMemory(i32, HardwareRegister),
    OrInt32Constant(HardwareRegister, i32),
    OrInt32ConstantToFrameMemory(i32, i32),

    NotInt32(HardwareRegister),
    NotInt32FrameMemory(i32),

    AddFloat32(HardwareRegister, HardwareRegister),
    AddFloat32FromFrameMemory(HardwareRegister, i32),

    SubFloat32(HardwareRegister, HardwareRegister),
    SubFloat32FromFrameMemory(HardwareRegister, i32),

    MultiplyFloat32(HardwareRegister, HardwareRegister),
    MultiplyFloat32FromFrameMemory(HardwareRegister, i32),

    DivideFloat32(HardwareRegister, HardwareRegister),
    DivideFloat32FromFrameMemory(HardwareRegister, i32),

    Call(FunctionSignature, Vec<Variable>, usize),
    Return,

    NullReferenceCheck(HardwareRegister),
    ArrayBoundsCheck(HardwareRegister, HardwareRegister),

    NewArray(TypeId, HardwareRegister, usize),
    LoadElement(TypeId, HardwareRegister, HardwareRegister, HardwareRegister),
    StoreElement(TypeId, HardwareRegister, HardwareRegister, HardwareRegister),
    LoadArrayLength(HardwareRegister, HardwareRegister),

    NewObject(TypeId),
    LoadField(TypeId, usize, HardwareRegister, HardwareRegister),
    StoreField(TypeId, usize, HardwareRegister, HardwareRegister),

    Compare(TypeId, HardwareRegister, HardwareRegister),
    CompareFromFrameMemory(TypeId, HardwareRegister, i32),
    CompareToFrameMemory(TypeId, i32, HardwareRegister),

    BranchLabel(BranchLabel),
    Branch(BranchLabel),
    BranchCondition(Condition, bool, BranchLabel),

    CompareResult(Condition, bool, HardwareRegister),

    PrintStackFrame(usize),
    GarbageCollect(usize)
}
