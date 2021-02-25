use crate::model::function::FunctionSignature;

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
    Return
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
        }
    }
}