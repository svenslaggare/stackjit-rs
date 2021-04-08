use crate::model::function::FunctionSignature;
use crate::model::typesystem::TypeId;

pub type BranchTarget = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    LoadInt32(i32),
    LoadFloat32(f32),
    LoadTrue,
    LoadFalse,
    LoadNull(TypeId),
    LoadLocal(u32),
    StoreLocal(u32),
    Add,
    Sub,
    Multiply,
    Divide,
    And,
    Or,
    Not,
    Call(FunctionSignature),
    LoadArgument(u32),
    Return,
    NewArray(TypeId),
    LoadElement(TypeId),
    StoreElement(TypeId),
    LoadArrayLength,
    NewObject(String),
    LoadField(String, String),
    StoreField(String, String),
    Branch(BranchTarget),
    BranchEqual(BranchTarget),
    BranchNotEqual(BranchTarget),
    BranchGreaterThan(BranchTarget),
    BranchGreaterThanOrEqual(BranchTarget),
    BranchLessThan(BranchTarget),
    BranchLessThanOrEqual(BranchTarget),
    CompareEqual,
    CompareNotEqual,
    CompareGreaterThan,
    CompareGreaterThanOrEqual,
    CompareLessThan,
    CompareLessThanOrEqual
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
            Instruction::LoadTrue => {
                write!(f, "LoadTrue")
            }
            Instruction::LoadFalse => {
                write!(f, "LoadFalse")
            }
            Instruction::LoadNull(null_type) => {
                write!(f, "LoadNull {}", null_type)
            }
            Instruction::LoadLocal(index) => {
                write!(f, "LoadLocal {}", index)
            }
            Instruction::StoreLocal(index) => {
                write!(f, "StoreLocal {}", index)
            }
            Instruction::Add => {
                write!(f, "Add")
            }
            Instruction::Sub => {
                write!(f, "Sub")
            }
            Instruction::Multiply => {
                write!(f, "Multiply")
            }
            Instruction::Divide => {
                write!(f, "Divide")
            }
            Instruction::And => {
                write!(f, "And")
            }
            Instruction::Or => {
                write!(f, "Or")
            }
            Instruction::Not => {
                write!(f, "Not")
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
            Instruction::NewObject(class_type) => {
                write!(f, "NewObject {}", class_type)
            }
            Instruction::LoadField(class_type, field) => {
                write!(f, "LoadField {}::{}", class_type, field)
            }
            Instruction::StoreField(class_type, field) => {
                write!(f, "StoreField {}::{}", class_type, field)
            }
            Instruction::LoadArrayLength => {
                write!(f, "LoadArrayLength")
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
            Instruction::CompareEqual => {
                write!(f, "CompareEqual")
            }
            Instruction::CompareNotEqual => {
                write!(f, "CompareNotEqual")
            }
            Instruction::CompareGreaterThan => {
                write!(f, "CompareGreaterThan")
            }
            Instruction::CompareGreaterThanOrEqual => {
                write!(f, "CompareGreaterThanOrEqual")
            }
            Instruction::CompareLessThan => {
                write!(f, "CompareLessThan")
            }
            Instruction::CompareLessThanOrEqual => {
                write!(f, "CompareLessThanOrEqual")
            }
        }
    }
}