use iced_x86::{Code, Encoder, MemoryOperand, Register};
use iced_x86::Instruction as X86Instruction;

use crate::compiler::{FunctionCallType, FunctionCompilationData, UnresolvedFunctionCall, stack_layout};
use crate::engine::binder::Binder;
use crate::compiler::calling_conventions::{CallingConventions, register_call_arguments};
use crate::ir::{HardwareRegisterExplicit, InstructionIR};
use crate::model::function::{Function, FunctionType};
use crate::runtime::runtime_interface;
use crate::model::typesystem::{TypeStorage, Type};

pub struct CodeGenerator<'a> {
    encoder: Encoder,
    encode_offset: usize,
    binder: &'a Binder,
    type_storage: &'a mut TypeStorage
}

impl<'a> CodeGenerator<'a> {
    pub fn new(binder: &'a Binder, type_storage: &'a mut TypeStorage) -> CodeGenerator<'a> {
        CodeGenerator {
            encoder: Encoder::new(64),
            encode_offset: 0,
            binder,
            type_storage
        }
    }

    pub fn generate(&mut self,
                    function: &Function,
                    compilation_data: &mut FunctionCompilationData,
                    instructions: &Vec<InstructionIR>) {
        for instruction in instructions {
            match instruction {
                InstructionIR::Marker(index) => {
                    println!("{}", function.instructions()[*index]);
                }
                _ => {
                    println!("\t{:?}", instruction);
                }
            }

            self.generate_instruction(&function, compilation_data, instruction);
        }
    }

    fn generate_instruction(&mut self,
                            function: &Function,
                            compilation_data: &mut FunctionCompilationData,
                            instruction: &InstructionIR) {
        match instruction {
            InstructionIR::Marker(_) => {},
            InstructionIR::InitializeFunction => {
                // Save the base pointer
                self.encode_x86_instruction(X86Instruction::with_reg(Code::Push_r64, Register::RBP));
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, Register::RBP, Register::RSP));
            },
            InstructionIR::LoadInt32(value) => {
                let push_instruction = compilation_data.operand_stack.push_i32(function, *value);
                self.generate_instruction(
                    function,
                    compilation_data,
                    &push_instruction
                );
            }
            InstructionIR::LoadZeroToRegister(register) => {
                let register = register_mapping::get(*register, true);
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Xor_r64_rm64, register, register));
            }
            InstructionIR::AddToStackPointer(value) => {
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(
                    Code::Add_rm64_imm32,
                    Register::RSP,
                    *value
                ).unwrap());
            }
            InstructionIR::SubFromStackPointer(value) => {
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(
                    Code::Sub_rm64_imm32,
                    Register::RSP,
                    *value
                ).unwrap());
            }
            InstructionIR::PushOperand(register) => {
                self.push_register_operand_stack(function, compilation_data, register_mapping::get(*register, true));
            }
            InstructionIR::PopOperand(register) => {
                self.pop_register_operand_stack(function, compilation_data, register_mapping::get(*register, true));
            }
            InstructionIR::PushOperandExplicit(register) => {
                self.push_register_operand_stack(function, compilation_data, register.0);
            }
            InstructionIR::PopOperandExplicit(register) => {
                self.pop_register_operand_stack(function, compilation_data, register.0);
            }
            InstructionIR::PushNormalExplicit(register) => {
                push_r64(
                    |instruction| self.encode_x86_instruction(instruction),
                    register.0
                );
            }
            InstructionIR::PopNormalExplicit(register) => {
                pop_r64(
                    |instruction| self.encode_x86_instruction(instruction),
                    register.0
                );
            }
            InstructionIR::LoadMemory(destination, offset) => {
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Mov_r64_rm64,
                    register_mapping::get(*destination, true),
                    MemoryOperand::with_base_displ(Register::RBP, *offset)
                ));
            }
            InstructionIR::StoreMemory(offset, source) => {
                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                    Code::Mov_rm64_r64,
                    MemoryOperand::with_base_displ(Register::RBP, *offset),
                    register_mapping::get(*source, true)
                ));
            }
            InstructionIR::StoreMemoryExplicit(offset, register) => {
                if register.0.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_mem_reg(
                        Code::Movss_xmmm32_xmm,
                        MemoryOperand::with_base_displ(Register::RBP, *offset),
                        register.0
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_mem_reg(
                        Code::Mov_rm64_r64,
                        MemoryOperand::with_base_displ(Register::RBP, *offset),
                        register.0
                    ));
                }
            }
            InstructionIR::LoadMemoryExplicit(register, offset) => {
                if register.0.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_mem(
                        Code::Movss_xmm_xmmm32,
                        register.0,
                        MemoryOperand::with_base_displ(Register::RBP, *offset)
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_mem(
                        Code::Mov_r64_rm64,
                        register.0,
                        MemoryOperand::with_base_displ(Register::RBP, *offset)
                    ));
                }
            }
            InstructionIR::MoveInt32ToMemory(offset, value) => {
                self.encode_x86_instruction(X86Instruction::try_with_mem_i32(
                    Code::Mov_rm64_imm32,
                    MemoryOperand::with_base_displ(Register::RBP, *offset),
                    *value
                ).unwrap());
            }
            InstructionIR::AddInt32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Add_r32_rm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::SubInt32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Sub_r32_rm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::AddFloat32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Addss_xmm_xmmm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::SubFloat32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Subss_xmm_xmmm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::Call(signature) => {
                let func_to_call = self.binder.get(signature).unwrap();
                let calling_conventions = CallingConventions::new();

                //Align the stack
                let stack_alignment = calling_conventions.stack_alignment(func_to_call);
                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, -stack_alignment).unwrap());
                }

                let mut call_argument_instructions = Vec::new();
                calling_conventions.call_function_arguments(
                    function,
                    compilation_data,
                    signature,
                    &mut call_argument_instructions
                );
                self.generate_instructions(function, compilation_data, &call_argument_instructions);

                match func_to_call.function_type() {
                    FunctionType::External => {
                        call_direct(
                            |instruction| self.encode_x86_instruction(instruction),
                            func_to_call.address().unwrap() as u64
                        )
                    }
                    FunctionType::Managed => {
                        compilation_data.unresolved_function_calls.push(UnresolvedFunctionCall {
                            call_type: FunctionCallType::Relative,
                            call_offset: self.encode_offset,
                            signature: signature.clone()
                        });

                        self.encode_x86_instruction(X86Instruction::try_with_branch(
                            Code::Call_rel32_64,
                            0
                        ).unwrap());
                    }
                }

                //Unalign the stack
                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, stack_alignment).unwrap());
                }

                //If we have passed arguments via the stack, adjust the stack pointer.
                let num_stack_arguments = calling_conventions.num_stack_arguments(func_to_call.parameters());
                if num_stack_arguments > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(
                        Code::Add_rm64_imm32,
                        Register::RSP,
                        num_stack_arguments as i32 * stack_layout::STACK_ENTRY_SIZE
                    ).unwrap());
                }
            }
            InstructionIR::Return => {
                //Restore the base pointer
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, Register::RSP, Register::RBP));
                self.encode_x86_instruction(X86Instruction::with_reg(Code::Pop_rm64, Register::RBP));

                self.encode_x86_instruction(X86Instruction::with(Code::Retnq));
            }
            InstructionIR::NewArray(element) => {
                let array_type = Type::Array(Box::new(element.clone()));
                let array_type_id = self.type_storage.add_or_get_type(array_type);

                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Mov_rm64_imm32, register_call_arguments::ARG0, array_type_id.0).unwrap());
                self.pop_register_operand_stack(function, compilation_data, register_call_arguments::ARG1);

                call_direct(
                    |instruction| self.encode_x86_instruction(instruction),
                    runtime_interface::new_array as u64
                );

                self.push_register_operand_stack(function, compilation_data, Register::RAX);
            }
            InstructionIR::LoadElement(element) => {

            }
            InstructionIR::StoreElement(element) => {

            }
        }
    }

    fn generate_instructions(&mut self,
                             function: &Function,
                             compilation_data: &mut FunctionCompilationData,
                             instructions: &Vec<InstructionIR>) {
        for instruction in instructions {
            self.generate_instruction(function, compilation_data, instruction);
        }
    }

    fn push_register_operand_stack(&mut self,
                                       function: &Function,
                                       compilation_data: &mut FunctionCompilationData,
                                       register: Register) {
        let instruction = compilation_data.operand_stack.push_register(function, HardwareRegisterExplicit(register));
        self.generate_instruction(
            function,
            compilation_data,
            &instruction
        );
    }

    fn pop_register_operand_stack(&mut self,
                                       function: &Function,
                                       compilation_data: &mut FunctionCompilationData,
                                       register: Register) {
        let instruction = compilation_data.operand_stack.pop_register(function, HardwareRegisterExplicit(register));
        self.generate_instruction(
            function,
            compilation_data,
            &instruction
        );
    }

    pub fn done(mut self) -> Vec<u8> {
        self.encoder.take_buffer()
    }

    pub fn encode_x86_instruction(&mut self, instruction: X86Instruction) {
        println!("\t\t{}", instruction);
        self.encode_offset += self.encoder.encode(&instruction, 0).unwrap();
    }
}

mod register_mapping {
    use iced_x86::Register;

    use crate::ir::HardwareRegister;

    lazy_static! {
       static ref mapping_i64: Vec<Register> = {
           vec![
                Register::R10,
                Register::R11,
                Register::R12,
                Register::R13,
                Register::R14,
            ]
       };

        static ref mapping_i32: Vec<Register> = {
            vec![
                Register::R10D,
                Register::R11D,
                Register::R12D,
                Register::R13D,
                Register::R14D,
            ]
        };

        static ref mapping_f32: Vec<Register> = {
            vec![
                Register::XMM1,
                Register::XMM2,
                Register::XMM3,
                Register::XMM4,
                Register::XMM5,
            ]
        };
    }

    pub fn get(register: HardwareRegister, is_64: bool) -> Register {
        match register {
            HardwareRegister::Int(index) => {
                if is_64 {
                    mapping_i64[index as usize]
                } else {
                    mapping_i32[index as usize]
                }
            }
            HardwareRegister::Float(index) => {
                mapping_f32[index as usize]
            }
        }
    }
}

fn call_direct<F: FnMut(X86Instruction)>(mut encode_instruction: F, address: u64) {
    encode_instruction(X86Instruction::try_with_reg_u64(Code::Mov_r64_imm64, Register::RAX, address).unwrap());
    encode_instruction(X86Instruction::with_reg(Code::Call_rm64, Register::RAX));
}

pub fn push_r32<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
    encode_instruction(X86Instruction::with_mem_reg(Code::Mov_rm32_r32, MemoryOperand::with_base(Register::RSP), register));
}

pub fn push_r64<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
    encode_instruction(X86Instruction::with_mem_reg(Code::Mov_rm64_r64, MemoryOperand::with_base(Register::RSP), register));
}

pub fn pop_r32<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::with_reg_mem(Code::Mov_r32_rm32, register, MemoryOperand::with_base(Register::RSP)));
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
}

pub fn pop_r64<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::with_reg_mem(Code::Mov_r64_rm64, register, MemoryOperand::with_base(Register::RSP)));
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
}