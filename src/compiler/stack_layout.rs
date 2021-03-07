use crate::model::function::Function;
use crate::ir::mid::compiler::MIRCompilationResult;

pub const STACK_ENTRY_SIZE: i32 = 8;
pub const STACK_OFFSET: u32 = 1;

pub fn stack_size(function: &Function) -> i32 {
    (function.definition().parameters().len() + function.locals().len() + function.operand_stack_size()) as i32 * STACK_ENTRY_SIZE
}

pub fn stack_size_mir(function: &Function, mir_result: &MIRCompilationResult) -> i32 {
    (function.definition().parameters().len() + mir_result.num_virtual_registers) as i32 * STACK_ENTRY_SIZE
}

pub fn align_size(size: i32) -> i32 {
    ((size + 15) / 16) * 16
}

pub fn argument_stack_offset(_function: &Function, index: u32) -> i32 {
    (STACK_OFFSET + index) as i32 * -STACK_ENTRY_SIZE
}

pub fn local_stack_offset(function: &Function, index: u32) -> i32 {
    (STACK_OFFSET + function.definition().parameters().len() as u32 + index) as i32 * -STACK_ENTRY_SIZE
}

pub fn operand_stack_offset(function: &Function, index: u32) -> i32 {
    (STACK_OFFSET as i32 + function.definition().parameters().len() as i32 + function.locals().len() as i32 + index as i32) * -STACK_ENTRY_SIZE
}

pub fn virtual_register_stack_offset(function: &Function, number: u32) -> i32 {
    -STACK_ENTRY_SIZE * (STACK_OFFSET + function.definition().parameters().len() as u32 + number) as i32
}
