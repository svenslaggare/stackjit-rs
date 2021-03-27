use crate::model::function::Function;
use crate::mir::compiler::MIRCompilationResult;

pub const STACK_ENTRY_SIZE: i32 = 8;
pub const STACK_OFFSET: u32 = 1;
pub const STACK_VALUE_OFFSET: u32 = STACK_OFFSET + 1;

pub fn stack_size(function: &Function, compilation_result: &MIRCompilationResult) -> i32 {
    align_size(needed_stack_size(function, compilation_result))
}

pub fn needed_stack_size(function: &Function, compilation_result: &MIRCompilationResult) -> i32 {
    (1 + function.definition().parameters().len() + compilation_result.num_virtual_registers) as i32 * STACK_ENTRY_SIZE
}

pub fn align_size(size: i32) -> i32 {
    ((size + 15) / 16) * 16
}

pub fn argument_stack_offset(_function: &Function, index: u32) -> i32 {
    (STACK_VALUE_OFFSET + index) as i32 * -STACK_ENTRY_SIZE
}

pub fn virtual_register_stack_offset(function: &Function, number: u32) -> i32 {
    -STACK_ENTRY_SIZE * (STACK_VALUE_OFFSET + function.definition().parameters().len() as u32 + number) as i32
}
