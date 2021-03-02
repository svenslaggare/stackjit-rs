use crate::model::function::FunctionSignature;
use crate::model::typesystem::Type;

pub type BranchTarget = u32;

#[derive(Debug, Clone)]
pub enum Instruction {
    LoadInt32(i32),
    LoadFloat32(f32),
    LoadLocal(u32),
    StoreLocal(u32),
    Add,
    Sub,
    Call(FunctionSignature),
    LoadArgument(u32),
    Return,
    NewArray(Type),
    LoadElement(Type),
    StoreElement(Type),
    Branch(BranchTarget),
    BranchEqual(BranchTarget),
    BranchNotEqual(BranchTarget),
    BranchGreaterThan(BranchTarget),
    BranchGreaterThanOrEqual(BranchTarget),
    BranchLessThan(BranchTarget),
    BranchLessThanOrEqual(BranchTarget)
}

impl Instruction {
    pub fn branch_target(&self) -> Option<BranchTarget> {
        match self {
            Instruction::Branch(target)
            | Instruction::BranchEqual(target)
            | Instruction::BranchNotEqual(target)
            | Instruction::BranchGreaterThan(target)
            | Instruction::BranchGreaterThanOrEqual(target)
            | Instruction::BranchLessThan(target)
            | Instruction::BranchLessThanOrEqual(target) => {
                Some(*target)
            }
            _ => None
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::LoadInt32(value) => {
                write!(f, "LoadInt32 {}", value)
            }
            Instruction::LoadFloat32(value) => {
                write!(f, "LoadFloat32 {}", value)
            }
            Instruction::LoadLocal(index) => {
                write!(f, "LoadLocal {}", index)
            }
            Instruction::StoreLocal(index) => {
                write!(f, "LoadLocal {}", index)
            }
            Instruction::Add => {
                write!(f, "Add")
            }
            Instruction::Sub => {
                write!(f, "Sub")
            }
            Instruction::Call(signature) => {
                write!(f, "Call {}", signature)
            }
            Instruction::LoadArgument(argument) => {
                write!(f, "LoadArgument {}", argument)
            }
            Instruction::Return => {
                write!(f, "Return")
            }
            Instruction::NewArray(element) => {
                write!(f, "NewArray {}", element)
            }
            Instruction::LoadElement(element) => {
                write!(f, "LoadElement {}", element)
            }
            Instruction::StoreElement(element) => {
                write!(f, "StoreElement {}", element)
            }
            Instruction::Branch(target) => {
                write!(f, "Branch {}", target)
            }
            Instruction::BranchEqual(target) => {
                write!(f, "BranchEqual {}", target)
            }
            Instruction::BranchNotEqual(target) => {
                write!(f, "BranchNotEqual {}", target)
            }
            Instruction::BranchGreaterThan(target) => {
                write!(f, "BranchGreaterThan {}", target)
            }
            Instruction::BranchGreaterThanOrEqual(target) => {
                write!(f, "BranchGreaterThanOrEqual {}", target)
            }
            Instruction::BranchLessThan(target) => {
                write!(f, "BranchLessThan {}", target)
            }
            Instruction::BranchLessThanOrEqual(target) => {
                write!(f, "BranchLessThanOrEqual {}", target)
            }
        }
    }
}