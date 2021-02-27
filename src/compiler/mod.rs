pub mod code_generator;
pub mod allocator;
pub mod binder;
pub mod stack_layout;
pub mod jit;
pub mod calling_conventions;

use crate::ir::{HardwareRegisterExplicit, InstructionIR};
use crate::model::function::{Function, FunctionSignature};

pub struct FunctionCompilationData {
    pub unresolved_function_calls: Vec<UnresolvedFunctionCall>,
    pub operand_stack: OperandStack
}

impl FunctionCompilationData {
    pub fn new() -> FunctionCompilationData {
        FunctionCompilationData {
            unresolved_function_calls: Vec::new(),
            operand_stack: OperandStack::new()
        }
    }
}

pub enum FunctionCallType {
    Relative,
    Absolute
}

pub struct UnresolvedFunctionCall {
    pub call_type: FunctionCallType,
    pub call_offset: usize,
    pub signature: FunctionSignature
}

pub struct OperandStack {
    top_index: Option<usize>
}

impl OperandStack {
    pub fn new() -> OperandStack {
        OperandStack {
            top_index: None
        }
    }

    pub fn push_register(&mut self,
                         function: &Function,
                         register: HardwareRegisterExplicit) -> InstructionIR {
        self.top_index = self.top_index.map_or(Some(0), |index| Some(index + 1));

        let stack_frame_offset = stack_layout::operand_stack_offset(function, self.top_index.unwrap() as u32);
        InstructionIR::StoreMemoryExplicit(stack_frame_offset, register)
    }

    pub fn push_i32(&mut self,
                    function: &Function,
                    value: i32) -> InstructionIR {
        self.top_index = self.top_index.map_or(Some(0), |index| Some(index + 1));
        let stack_frame_offset = stack_layout::operand_stack_offset(function, self.top_index.unwrap() as u32);
        InstructionIR::MoveInt32ToMemory(stack_frame_offset, value)
    }

    pub fn pop_register(&mut self,
                        function: &Function,
                        register: HardwareRegisterExplicit) -> InstructionIR {
        let top_index = self.top_index.unwrap();

        let stack_frame_offset = stack_layout::operand_stack_offset(function, top_index as u32);
        let instruction = InstructionIR::LoadMemoryExplicit(register, stack_frame_offset);

        if let Some(0) = self.top_index {
            self.top_index = None;
        } else {
            *self.top_index.as_mut().unwrap() -= 1;
        }

        instruction
    }
}