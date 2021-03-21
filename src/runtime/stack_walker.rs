use crate::compiler::{FunctionCompilationData, stack_layout};
use crate::model::function::Function;
use crate::compiler::jit::JitCompiler;
use crate::engine::binder::Binder;
use crate::ir::mid::RegisterMIR;
use crate::model::typesystem::Type;

pub struct StackFrame<'a> {
    base_pointer: u64,
    instruction_index: usize,
    function: &'a Function,
    compiled_function: &'a FunctionCompilationData
}

impl<'a> StackFrame<'a> {
    pub fn new(base_pointer: u64,
               instruction_index: usize,
               function: &'a Function,
               compiled_function: &'a FunctionCompilationData) -> StackFrame<'a> {
        StackFrame {
            base_pointer,
            instruction_index,
            function,
            compiled_function
        }
    }

    pub fn parent(&self, compiler: &'a JitCompiler, binder: &'a Binder) -> Option<StackFrame<'a>> {
        if self.function.definition().name() == "main" {
            return None;
        }

        let parent_base_pointer = unsafe { *(self.base_pointer as *const u64) };
        let parent_function_address = unsafe { *((parent_base_pointer as isize - 8) as *const u64) };
        let parent_function = unsafe { (parent_function_address as *const Function).as_ref() }.unwrap();
        let parent_signature = parent_function.definition().call_signature();
        let parent_compiled_function = compiler
            .get_compiled_function(&parent_signature)
            .unwrap();

        let parent_function_code_ptr = binder.get(&parent_signature).unwrap().address().unwrap();

        let parent_call_point_address = unsafe { *((self.base_pointer as isize + 8) as *const u64) } as isize;
        let parent_call_offset = (parent_call_point_address - parent_function_code_ptr as isize) as usize;
        let parent_call_instruction_index = instruction_index_from_offset(parent_compiled_function, parent_call_offset)?;

        Some(
            StackFrame::new(
                parent_base_pointer,
                parent_call_instruction_index,
                parent_function,
                parent_compiled_function
            )
        )
    }

    pub fn walk<F: FnMut(&StackFrame<'a>)>(&self,
                                           compiler: &'a JitCompiler,
                                           binder: &'a Binder,
                                           mut apply: F) {
        apply(self);
        let mut parent_frame = self.parent(compiler, binder);

        while let Some(frame) = parent_frame.take() {
            apply(&frame);
            parent_frame = frame.parent(compiler, binder);
        }
    }

    pub fn arguments(&'a self) -> StackFrameArgumentsIterator {
        StackFrameArgumentsIterator::new(self)
    }

    pub fn locals(&'a self) -> StackFrameLocalsIterator {
        StackFrameLocalsIterator::new(self)
    }

    pub fn operands(&'a self) -> StackFrameOperandsIterator {
        StackFrameOperandsIterator::new(self)
    }

    pub fn print_frame(&self) {
        println!("{} @ {}", self.function.definition().signature(), self.instruction_index);

        println!("\tArguments:");
        for value in self.arguments() {
            println!("\t{}", value);
        }

        println!();

        println!("\tLocals:");
        for value in self.locals() {
            println!("\t{}", value);
        }

        println!();

        println!("\tOperands:");
        for value in self.operands() {
            println!("\t{}", value);
        }
    }
}

pub struct StackFrameArgumentsIterator<'a> {
    stack_frame: &'a StackFrame<'a>,
    index: usize
}

impl<'a> StackFrameArgumentsIterator<'a> {
    pub fn new(stack_frame: &'a StackFrame<'a>) -> StackFrameArgumentsIterator<'a> {
        StackFrameArgumentsIterator {
            stack_frame,
            index: 0
        }
    }

    fn arguments(&self) -> &'a Vec<Type> {
        &self.stack_frame.function.definition().parameters()
    }
}

impl<'a> Iterator for StackFrameArgumentsIterator<'a> {
    type Item = FrameValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.arguments().len() {
            return None;
        }

        let argument_type = &self.arguments()[self.index];
        let value_offset = stack_layout::argument_stack_offset(self.stack_frame.function, self.index as u32);
        let value_ptr = (self.stack_frame.base_pointer as isize + value_offset as isize) as *const u8;

        self.index += 1;
        Some(FrameValue::new_value(argument_type, value_ptr))
    }
}

pub struct StackFrameLocalsIterator<'a> {
    stack_frame: &'a StackFrame<'a>,
    index: usize
}

impl<'a> StackFrameLocalsIterator<'a> {
    pub fn new(stack_frame: &'a StackFrame<'a>) -> StackFrameLocalsIterator<'a> {
        StackFrameLocalsIterator {
            stack_frame,
            index: 0
        }
    }

    fn locals(&self) -> &'a Vec<RegisterMIR> {
        &self.stack_frame.compiled_function.mir_compilation_result.local_virtual_registers
    }
}

impl<'a> Iterator for StackFrameLocalsIterator<'a> {
    type Item = FrameValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.locals().len() {
            return None;
        }

        let register = &self.locals()[self.index];
        let value_offset = stack_layout::virtual_register_stack_offset(self.stack_frame.function, register.number);
        let value_ptr = (self.stack_frame.base_pointer as isize + value_offset as isize) as *const u8;

        self.index += 1;
        Some(FrameValue::new_register(&register.value_type, register, value_ptr))
    }
}

pub struct StackFrameOperandsIterator<'a> {
    stack_frame: &'a StackFrame<'a>,
    index: usize
}

impl<'a> StackFrameOperandsIterator<'a> {
    pub fn new(stack_frame: &'a StackFrame<'a>) -> StackFrameOperandsIterator<'a> {
        StackFrameOperandsIterator {
            stack_frame,
            index: 0
        }
    }

    fn operand_registers(&self) -> &'a Vec<RegisterMIR> {
        &self.stack_frame.compiled_function.mir_compilation_result.instructions_operand_types[self.stack_frame.instruction_index]
    }
}

impl<'a> Iterator for StackFrameOperandsIterator<'a> {
    type Item = FrameValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.operand_registers().len() {
            return None;
        }

        let register = &self.operand_registers()[self.index];
        let value_offset = stack_layout::virtual_register_stack_offset(self.stack_frame.function, register.number);
        let value_ptr = (self.stack_frame.base_pointer as isize + value_offset as isize) as *const u8;

        self.index += 1;
        Some(FrameValue::new_register(&register.value_type, register, value_ptr))
    }
}

pub struct FrameValue<'a> {
    pub value_type: &'a Type,
    pub register: Option<&'a RegisterMIR>,
    value_ptr: *const u8
}

impl<'a> FrameValue<'a> {
    pub fn new_value(value_type: &'a Type, value_ptr: *const u8) -> FrameValue<'a> {
        FrameValue {
            register: None,
            value_type,
            value_ptr
        }
    }

    pub fn new_register(value_type: &'a Type, register: &'a RegisterMIR, value_ptr: *const u8) -> FrameValue<'a> {
        FrameValue {
            register: Some(register),
            value_type,
            value_ptr
        }
    }

    pub fn value_u64(&self) -> u64 {
        unsafe { *(self.value_ptr as *const u64) }
    }
}

impl<'a> std::fmt::Display for FrameValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(register) = self.register {
            write!(f, "{:?}", register)?;
        } else {
            write!(f, "{:?}", self.value_type)?;
        }

        write!(f, ": ")?;

        match self.value_type {
            Type::Void => {
                write!(f, "()")
            }
            Type::Int32 => {
                write!(f, "{}", self.value_u64())
            }
            Type::Float32 => {
                write!(f, "{}", self.value_u64())
            }
            Type::Array(_) => {
                write!(f, "0x{:0x}", self.value_u64())
            }
        }
    }
}

fn instruction_index_from_offset(compiled_function: &FunctionCompilationData, offset: usize) -> Option<usize> {
    for index in 0..compiled_function.instructions_offsets.len() {
        if index + 1 < compiled_function.instructions_offsets.len() {
            if offset >= compiled_function.instructions_offsets[index].1 && offset <= compiled_function.instructions_offsets[index + 1].1 {
                return Some(compiled_function.instructions_offsets[index].0);
            }
        }
    }

    return None;
}