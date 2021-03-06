use iced_x86::{Encoder, Code, Register, MemoryOperand};
use iced_x86::Instruction as X86Instruction;

use crate::compiler::calling_conventions::register_call_arguments;
use crate::runtime::runtime_interface;
use crate::compiler::allocator::ExecutableMemoryAllocator;

pub struct ErrorHandling {
    pub null_check_handler: *const std::ffi::c_void,
    pub array_create_check_handler: *const std::ffi::c_void,
    pub array_bounds_check_handler: *const std::ffi::c_void
}

impl ErrorHandling {
    pub fn new(memory_allocator: &mut ExecutableMemoryAllocator) -> ErrorHandling {
        // Create handler calls
        let mut encoder = Encoder::new(64);
        let null_check_handler_offset = ErrorHandling::generate_handler(&mut encoder, runtime_interface::null_error as u64);
        let array_create_check_handler_offset = ErrorHandling::generate_handler(&mut encoder, runtime_interface::array_create_error as u64);
        let array_bounds_check_handler_offset = ErrorHandling::generate_handler(&mut encoder, runtime_interface::array_bounds_error as u64);

        // Allocate and copy memory
        let handler_buffer = encoder.take_buffer();
        let handler_ptr = memory_allocator.allocate(handler_buffer.len());
        unsafe {
            handler_ptr.copy_from(handler_buffer.as_ptr() as *const _, handler_buffer.len());
        }

        ErrorHandling {
            null_check_handler: unsafe { handler_ptr.add(null_check_handler_offset) },
            array_create_check_handler: unsafe { handler_ptr.add(array_create_check_handler_offset) },
            array_bounds_check_handler: unsafe { handler_ptr.add(array_bounds_check_handler_offset) }
        }
    }

    fn generate_handler(encoder: &mut Encoder, handler_function_address: u64) -> usize {
        let buffer = encoder.take_buffer();
        let handler_offset = buffer.len();
        encoder.set_buffer(buffer);

        let mut encode_x86_instruction = |instruction: X86Instruction| {
            encoder.encode(&instruction, 0).unwrap();
        };

        // Invoke the handler with a pointer to current stack that will have the return values
        encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, 32).unwrap());
        encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_rm64_r64, register_call_arguments::ARG0, Register::RSP));

        encode_x86_instruction(X86Instruction::try_with_reg_u64(Code::Mov_r64_imm64, Register::RAX, handler_function_address).unwrap());
        encode_x86_instruction(X86Instruction::with_reg(Code::Call_rm64, Register::RAX));

        // Return address (entrypoint invoker)
        encode_x86_instruction(X86Instruction::with_reg_mem(
            Code::Mov_r64_rm64,
            Register::RDI,
            MemoryOperand::with_base_displ(Register::RSP, 0)
        ));

        // Base & stack pointer when entrypoint was called
        encode_x86_instruction(X86Instruction::with_reg_mem(
            Code::Mov_r64_rm64,
            Register::RBP,
            MemoryOperand::with_base_displ(Register::RSP, 8)
        ));

        encode_x86_instruction(X86Instruction::with_reg_mem(
            Code::Mov_r64_rm64,
            Register::RSP,
            MemoryOperand::with_base_displ(Register::RSP, 16)
        ));

        // Simulate return instruction with custom address
        encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, 8).unwrap());
        encode_x86_instruction(X86Instruction::with_reg(Code::Jmp_rm64, Register::RDI));

        handler_offset
    }
}