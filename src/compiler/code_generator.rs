use iced_x86::{Code, Encoder, MemoryOperand, Register};
use iced_x86::Instruction as X86Instruction;

use crate::compiler::{FunctionCallType, FunctionCompilationData, stack_layout, UnresolvedFunctionCall};
use crate::compiler::calling_conventions::{CallingConventions, float_register_call_arguments, register_call_arguments};
use crate::compiler::error_handling::ErrorHandling;
use crate::compiler::ir::{Condition, HardwareRegisterExplicit, InstructionIR};
use crate::engine::binder::Binder;
use crate::model::function::{Function, FunctionType};
use crate::model::typesystem::{Type, TypeMetadata, TypeStorage};
use crate::runtime::{array, runtime_interface};

pub struct CodeGeneratorResult {
    pub code_bytes: Vec<u8>,
    pub instructions_offsets: Vec<(usize, usize)>
}

pub struct CodeGenerator<'a> {
    encoder: Encoder,
    encoder_offset: usize,
    binder: &'a Binder,
    error_handling: &'a ErrorHandling,
    type_storage: &'a mut TypeStorage,
    instructions_offsets: Vec<(usize, usize)>
}

impl<'a> CodeGenerator<'a> {
    pub fn new(binder: &'a Binder,
               error_handling: &'a ErrorHandling,
               type_storage: &'a mut TypeStorage) -> CodeGenerator<'a> {
        CodeGenerator {
            encoder: Encoder::new(64),
            encoder_offset: 0,
            binder,
            error_handling,
            type_storage,
            instructions_offsets: Vec::new()
        }
    }

    pub fn generate(&mut self,
                    function: &Function,
                    compilation_data: &mut FunctionCompilationData,
                    instructions: &Vec<InstructionIR>) {
        for instruction in instructions {
            match instruction {
                InstructionIR::Marker(index, _) => {
                    println!("{}", function.instructions()[*index]);
                }
                _ => {
                    println!("\t{:?}", instruction);
                }
            }

            self.generate_instruction(&function, compilation_data, instruction);
        }
    }

    pub fn done(mut self) -> CodeGeneratorResult {
        CodeGeneratorResult {
            code_bytes: self.encoder.take_buffer(),
            instructions_offsets: self.instructions_offsets
        }
    }

    fn generate_instruction(&mut self,
                            function: &Function,
                            compilation_data: &mut FunctionCompilationData,
                            instruction: &InstructionIR) {
        match instruction {
            InstructionIR::Marker(_, mir_instruction_index) => {
                self.instructions_offsets.push((*mir_instruction_index, self.encoder_offset));
            },
            InstructionIR::InitializeFunction => {
                let is_entry_point = function.definition().is_entry_point();
                if is_entry_point {
                    self.encode_x86_instruction(X86Instruction::with_reg_mem(
                        Code::Mov_r64_rm64,
                        register_call_arguments::ARG0,
                        MemoryOperand::with_base_displ(Register::RSP, 0)
                    ));

                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Mov_r64_rm64,
                        register_call_arguments::ARG1,
                        Register::RBP
                    ));

                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Mov_r64_rm64,
                        register_call_arguments::ARG2,
                        Register::RSP
                    ));
                }

                // Save the base pointer
                self.encode_x86_instruction(X86Instruction::with_reg(Code::Push_r64, Register::RBP));
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, Register::RBP, Register::RSP));

                if is_entry_point {
                    call_direct(
                        |instruction| self.encode_x86_instruction(instruction),
                        runtime_interface::set_error_return as u64
                    );
                }

                // Indicate which function that is being executed
                let function_address = function as *const _ as *const u64 as u64;
                self.encode_x86_instruction(X86Instruction::try_with_reg_u64(Code::Mov_r64_imm64, Register::RAX, function_address).unwrap());
                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                    Code::Mov_rm64_r64,
                    MemoryOperand::with_base_displ(Register::RBP, -(stack_layout::STACK_OFFSET as i32) * stack_layout::STACK_ENTRY_SIZE),
                    Register::RAX
                ));
            },
            InstructionIR::LoadZeroToRegister(register) => {
                let register = register_mapping::get(*register, true);

                if register.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Pxor_xmm_xmmm128, register, register));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Xor_r64_rm64, register, register));
                }
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
            InstructionIR::Push(register) => {
                let register = register_mapping::get(*register, true);

                if register.is_xmm() {
                    push_xmm(
                        |instruction| self.encode_x86_instruction(instruction),
                        register
                    );
                } else {
                    push_r64(
                        |instruction| self.encode_x86_instruction(instruction),
                        register
                    );
                }
            }
            InstructionIR::Pop(register) => {
                let register = register_mapping::get(*register, true);

                if register.is_xmm() {
                    pop_xmm(
                        |instruction| self.encode_x86_instruction(instruction),
                        register
                    );
                } else {
                    pop_r64(
                        |instruction| self.encode_x86_instruction(instruction),
                        register
                    );
                }
            }
            InstructionIR::PushExplicit(register) => {
                push_r64(
                    |instruction| self.encode_x86_instruction(instruction),
                    register.0
                );
            }
            InstructionIR::PopExplicit(register) => {
                pop_r64(
                    |instruction| self.encode_x86_instruction(instruction),
                    register.0
                );
            }
            InstructionIR::PopEmpty => {
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, Register::RAX.size() as i32).unwrap());
            }
            InstructionIR::PushInt32(value) => {
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, Register::RAX.size() as i32).unwrap());
                self.encode_x86_instruction(X86Instruction::try_with_mem_i32(
                    Code::Mov_rm64_imm32,
                    MemoryOperand::with_base(Register::RSP),
                    *value
                ).unwrap());
            }
            InstructionIR::LoadFrameMemory(destination, offset) => {
                let destination = register_mapping::get(*destination, true);

                if destination.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_mem(
                        Code::Movss_xmm_xmmm32,
                        destination,
                        MemoryOperand::with_base_displ(Register::RBP, *offset)
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_mem(
                        Code::Mov_r64_rm64,
                        destination,
                        MemoryOperand::with_base_displ(Register::RBP, *offset)
                    ));
                }
            }
            InstructionIR::StoreFrameMemory(offset, source) => {
                let source = register_mapping::get(*source, true);

                if source.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_mem_reg(
                        Code::Movss_xmmm32_xmm,
                        MemoryOperand::with_base_displ(Register::RBP, *offset),
                        source
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_mem_reg(
                        Code::Mov_rm64_r64,
                        MemoryOperand::with_base_displ(Register::RBP, *offset),
                        source
                    ));
                }
            }
            InstructionIR::StoreFrameMemoryExplicit(offset, register) => {
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
            InstructionIR::LoadFrameMemoryExplicit(register, offset) => {
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
            InstructionIR::MoveInt32ToFrameMemory(offset, value) => {
                self.encode_x86_instruction(X86Instruction::try_with_mem_i32(
                    Code::Mov_rm64_imm32,
                    MemoryOperand::with_base_displ(Register::RBP, *offset),
                    *value
                ).unwrap());
            }
            InstructionIR::MoveInt32ToRegister(destination, value) => {
                let destination = register_mapping::get(*destination, false);

                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(
                    Code::Mov_r32_imm32,
                    destination,
                    *value
                ).unwrap());
            }
            InstructionIR::Move(destination, source) => {
                let destination = register_mapping::get(*destination, true);
                let source = register_mapping::get(*source, true);

                if source.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Movss_xmm_xmmm32,
                        destination,
                        source
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Mov_r64_rm64,
                        destination,
                        source
                    ));
                }
            }
            InstructionIR::MoveImplicitToExplicit(destination, source) => {
                if destination.0.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Movss_xmm_xmmm32,
                        destination.0,
                        register_mapping::get(*source, true)
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Mov_r64_rm64,
                        destination.0,
                        register_mapping::get(*source, true)
                    ));
                }
            }
            InstructionIR::MoveExplicitToImplicit(destination, source) => {
                let destination = register_mapping::get(*destination, true);

                if destination.is_xmm() {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Movss_xmm_xmmm32,
                        destination,
                        source.0,
                    ));
                } else {
                    self.encode_x86_instruction(X86Instruction::with_reg_reg(
                        Code::Mov_rm64_r64,
                        destination,
                        source.0,
                    ));
                }
            }
            InstructionIR::AddInt32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Add_r32_rm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::AddInt32FromFrameMemory(destination, source_offset) => {
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Add_r32_rm32,
                    register_mapping::get(*destination, false),
                    MemoryOperand::with_base_displ(Register::RBP, *source_offset)
                ));
            }
            InstructionIR::AddInt32ToFrameMemory(destination_offset, source) => {
                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                    Code::Add_rm32_r32,
                    MemoryOperand::with_base_displ(Register::RBP, *destination_offset),
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
            InstructionIR::SubInt32FromFrameMemory(destination, source_offset) => {
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Sub_r32_rm32,
                    register_mapping::get(*destination, false),
                    MemoryOperand::with_base_displ(Register::RBP, *source_offset)
                ));
            }
            InstructionIR::SubInt32ToFrameMemory(destination_offset, source) => {
                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                    Code::Sub_rm32_r32,
                    MemoryOperand::with_base_displ(Register::RBP, *destination_offset),
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
            InstructionIR::AddFloat32FromFrameMemory(destination, source_offset) => {
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Addss_xmm_xmmm32,
                    register_mapping::get(*destination, false),
                    MemoryOperand::with_base_displ(Register::RBP, *source_offset)
                ));
            }
            InstructionIR::SubFloat32(destination, source) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(
                    Code::Subss_xmm_xmmm32,
                    register_mapping::get(*destination, false),
                    register_mapping::get(*source, false)
                ));
            }
            InstructionIR::SubFloat32FromFrameMemory(destination, source_offset) => {
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Subss_xmm_xmmm32,
                    register_mapping::get(*destination, false),
                    MemoryOperand::with_base_displ(Register::RBP, *source_offset)
                ));
            }
            InstructionIR::Call(signature, arguments, num_saved) => {
                let func_to_call = self.binder.get(signature).unwrap();
                let calling_conventions = CallingConventions::new();

                //Align the stack
                let stack_alignment = calling_conventions.stack_alignment(func_to_call, *num_saved);
                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, stack_alignment).unwrap());
                }

                let mut call_argument_instructions = Vec::new();
                calling_conventions.call_function_arguments(
                    signature,
                    &arguments,
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
                            call_offset: self.encoder_offset,
                            signature: signature.clone()
                        });

                        self.encode_x86_instruction(X86Instruction::try_with_branch(
                            Code::Call_rel32_64,
                            0
                        ).unwrap());
                    }
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

                //Unalign the stack
                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, stack_alignment).unwrap());
                }
            }
            InstructionIR::Return => {
                //Restore the base pointer
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, Register::RSP, Register::RBP));
                self.encode_x86_instruction(X86Instruction::with_reg(Code::Pop_rm64, Register::RBP));

                self.encode_x86_instruction(X86Instruction::with(Code::Retnq));
            }
            InstructionIR::NullReferenceCheck(reference_register) => {
                let reference_register = register_mapping::get(*reference_register, true);

                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Xor_r64_rm64, Register::RAX, Register::RAX));
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Cmp_r64_rm64, reference_register, Register::RAX));

                let instruction_size = self.encode_x86_instruction_with_size(X86Instruction::try_with_branch(Code::Je_rel32_64, 0).unwrap());
                compilation_data.unresolved_native_branches.insert(
                    self.encoder_offset - instruction_size,
                    self.error_handling.null_check_handler as usize
                );
            }
            InstructionIR::ArrayBoundsCheck(reference_register, index_register) => {
                let reference_register = register_mapping::get(*reference_register, true);
                let index_register = register_mapping::get(*index_register, true);

                self.encode_x86_instruction(X86Instruction::with_reg_mem(Code::Mov_r32_rm32, Register::EAX, MemoryOperand::with_base(reference_register))); // Array length
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Cmp_r64_rm64, index_register, Register::RAX));

                // By using an unsigned comparison, we only need one check.
                let instruction_size = self.encode_x86_instruction_with_size(X86Instruction::try_with_branch(Code::Jae_rel32_64, 0).unwrap());
                compilation_data.unresolved_native_branches.insert(
                    self.encoder_offset - instruction_size,
                    self.error_handling.array_bounds_check_handler as usize
                );
            }
            InstructionIR::NewArray(element, size_register, num_saved) => {
                let stack_alignment = (*num_saved as i32 % 2) * stack_layout::STACK_ENTRY_SIZE;
                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, stack_alignment).unwrap());
                }

                let array_type = Type::Array(Box::new(element.clone()));
                let array_type_holder = self.type_storage.entry(array_type);
                let array_type_holder = array_type_holder as *const TypeMetadata as *const u64 as u64;
                println!("0x{:0x}", array_type_holder);

                let size_register = register_mapping::get(*size_register, true);
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, register_call_arguments::ARG1, size_register));

                // Check that the size is valid
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Xor_r32_rm32, Register::EAX, Register::EAX));
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Cmp_r32_rm32, Register::EAX, register_call_arguments::ARG1.full_register32()));

                let instruction_size = self.encode_x86_instruction_with_size(X86Instruction::try_with_branch(Code::Jg_rel32_64, 0).unwrap());
                compilation_data.unresolved_native_branches.insert(
                    self.encoder_offset - instruction_size,
                    self.error_handling.array_create_check_handler as usize
                );

                self.encode_x86_instruction(X86Instruction::try_with_reg_u64(
                    Code::Mov_r64_imm64,
                    register_call_arguments::ARG0,
                    array_type_holder
                ).unwrap());

                call_direct(
                    |instruction| self.encode_x86_instruction(instruction),
                    runtime_interface::new_array as u64
                );

                if stack_alignment > 0 {
                    self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, stack_alignment).unwrap());
                }
            }
            InstructionIR::LoadElement(element, destination_register, reference_register, index_register) => {
                let reference_register = register_mapping::get(*reference_register, true);
                let index_register = register_mapping::get(*index_register, true);

                let memory_operand = self.compute_array_element_address(element, reference_register, index_register);

                // Load the element
                match element.size() {
                    8 => {
                        let destination_register = register_mapping::get(*destination_register, true);

                        self.encode_x86_instruction(X86Instruction::with_reg_mem(
                            Code::Mov_r64_rm64,
                            destination_register,
                            memory_operand,
                        ));
                    }
                    4 => {
                        let destination_register = register_mapping::get(*destination_register, false);

                        match element {
                            Type::Float32 => {
                                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                                    Code::Movss_xmm_xmmm32,
                                    destination_register,
                                    memory_operand,
                                ));
                            }
                            _ => {
                                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                                    Code::Mov_r32_rm32,
                                    destination_register,
                                    memory_operand,
                                ));
                            }
                        }
                    }
                    1 => {
                        unimplemented!();
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            InstructionIR::StoreElement(element, reference_register, index_register, value_register) => {
                let reference_register = register_mapping::get(*reference_register, true);
                let index_register = register_mapping::get(*index_register, true);

                let memory_operand = self.compute_array_element_address(element, reference_register, index_register);

                //Store the element
                match element.size() {
                    8 => {
                        let value_register = register_mapping::get(*value_register, true);
                        self.encode_x86_instruction(X86Instruction::with_mem_reg(
                            Code::Mov_rm64_r64,
                            memory_operand,
                            value_register,
                        ));
                    }
                    4 => {
                        match element {
                            Type::Float32 => {
                                let value_register = register_mapping::get(*value_register, true);
                                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                                    Code::Movss_xmmm32_xmm,
                                    memory_operand,
                                    value_register,
                                ));
                            }
                            _ => {
                                let value_register_32 = register_mapping::get(*value_register, false);
                                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                                    Code::Mov_rm32_r32,
                                    memory_operand,
                                    value_register_32,
                                ));
                            }
                        }
                    }
                    1 => {
                        unimplemented!();
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            InstructionIR::LoadArrayLength(destination_register, reference_register) => {
                let destination_register = register_mapping::get(*destination_register, false);
                let reference_register = register_mapping::get(*reference_register, true);
                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                    Code::Mov_r32_rm32,
                    destination_register,
                    MemoryOperand::with_base(reference_register)
                ));
            },
            InstructionIR::NewObject(class_type) => {
                let class_type = self.type_storage.entry(class_type.clone());
                let class_type_holder = class_type as *const TypeMetadata as *const u64 as u64;
                println!("0x{:0x}", class_type_holder);

                self.encode_x86_instruction(X86Instruction::try_with_reg_u64(
                    Code::Mov_r64_imm64,
                    register_call_arguments::ARG0,
                    class_type_holder
                ).unwrap());

                call_direct(
                    |instruction| self.encode_x86_instruction(instruction),
                    runtime_interface::new_class as u64
                );
            },
            InstructionIR::LoadField(field_type, field_offset, destination_register, reference_register) => {
                let reference_register = register_mapping::get(*reference_register, true);

                let memory_operand = MemoryOperand::with_base_displ(reference_register, *field_offset as i32);

                // Load the field
                match field_type.size() {
                    8 => {
                        let destination_register = register_mapping::get(*destination_register, true);

                        self.encode_x86_instruction(X86Instruction::with_reg_mem(
                            Code::Mov_r64_rm64,
                            destination_register,
                            memory_operand,
                        ));
                    }
                    4 => {
                        let destination_register = register_mapping::get(*destination_register, false);

                        match field_type {
                            Type::Float32 => {
                                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                                    Code::Movss_xmm_xmmm32,
                                    destination_register,
                                    memory_operand,
                                ));
                            }
                            _ => {
                                self.encode_x86_instruction(X86Instruction::with_reg_mem(
                                    Code::Mov_r32_rm32,
                                    destination_register,
                                    memory_operand,
                                ));
                            }
                        }
                    }
                    1 => {
                        unimplemented!();
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            InstructionIR::StoreField(field_type, field_offset, reference_register, value_register) => {
                let reference_register = register_mapping::get(*reference_register, true);

                let memory_operand = MemoryOperand::with_base_displ(reference_register, *field_offset as i32);

                //Store the field
                match field_type.size() {
                    8 => {
                        let value_register = register_mapping::get(*value_register, false);
                        self.encode_x86_instruction(X86Instruction::with_mem_reg(
                            Code::Mov_rm64_r64,
                            memory_operand,
                            value_register,
                        ));
                    }
                    4 => {
                        match field_type {
                            Type::Float32 => {
                                let value_register = register_mapping::get(*value_register, true);
                                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                                    Code::Movss_xmmm32_xmm,
                                    memory_operand,
                                    value_register,
                                ));
                            }
                            _ => {
                                let value_register_32 = register_mapping::get(*value_register, false);
                                self.encode_x86_instruction(X86Instruction::with_mem_reg(
                                    Code::Mov_rm32_r32,
                                    memory_operand,
                                    value_register_32,
                                ));
                            }
                        }
                    }
                    1 => {
                        unimplemented!();
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            InstructionIR::BranchLabel(label) => {
                compilation_data.branch_targets.insert(*label, self.encoder_offset);
            }
            InstructionIR::Branch(target) => {
                //As the exact target in native instructions is not known, defer to later.
                let instruction_size = self.encode_x86_instruction_with_size(X86Instruction::try_with_branch(Code::Jmp_rel32_64, 0).unwrap());
                compilation_data.unresolved_branches.insert(self.encoder_offset - instruction_size, (*target, instruction_size));
            }
            InstructionIR::Compare(op_type, op1, op2) => {
                let op1 = register_mapping::get(*op1, false);
                let op2 = register_mapping::get(*op2, false);

                match op_type {
                    Type::Float32 => {
                        self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Ucomiss_xmm_xmmm32, op1, op2));
                    }
                    _ => {
                        self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Cmp_r32_rm32, op1, op2));
                    }
                }
            }
            InstructionIR::CompareFromFrameMemory(op_type, op1, op2_offset) => {
                let op1 = register_mapping::get(*op1, false);

                match op_type {
                    Type::Float32 => {
                        self.encode_x86_instruction(X86Instruction::with_reg_mem(
                            Code::Ucomiss_xmm_xmmm32,
                            op1,
                            MemoryOperand::with_base_displ(Register::RBP, *op2_offset)
                        ));
                    }
                    _ => {
                        self.encode_x86_instruction(X86Instruction::with_reg_mem(
                            Code::Cmp_r32_rm32,
                            op1,
                            MemoryOperand::with_base_displ(Register::RBP, *op2_offset)
                        ));
                    }
                }
            }
            InstructionIR::CompareToFrameMemory(op_type, op1_offset, op2) => {
                let op2 = register_mapping::get(*op2, false);

                match op_type {
                    Type::Float32 => {
                        unimplemented!();
                    }
                    _ => {
                        self.encode_x86_instruction(X86Instruction::with_mem_reg(
                            Code::Cmp_rm32_r32,
                            MemoryOperand::with_base_displ(Register::RBP, *op1_offset),
                            op2
                        ));
                    }
                }
            }
            InstructionIR::BranchCondition(condition, signed, target) => {
                let compare_code = if !signed {
                    match condition {
                        Condition::Equal => Code::Je_rel32_64,
                        Condition::NotEqual => Code::Jne_rel32_64,
                        Condition::LessThan => Code::Jb_rel32_64,
                        Condition::LessThanOrEqual => Code::Jbe_rel32_64,
                        Condition::GreaterThan => Code::Ja_rel32_64,
                        Condition::GreaterThanOrEqual => Code::Jae_rel32_64
                    }
                } else {
                    match condition {
                        Condition::Equal => Code::Je_rel32_64,
                        Condition::NotEqual => Code::Jne_rel32_64,
                        Condition::LessThan => Code::Jl_rel32_64,
                        Condition::LessThanOrEqual => Code::Jle_rel32_64,
                        Condition::GreaterThan => Code::Jg_rel32_64,
                        Condition::GreaterThanOrEqual => Code::Jge_rel32_64
                    }
                };

                let instruction_size = self.encode_x86_instruction_with_size(X86Instruction::try_with_branch(compare_code, 0).unwrap());
                compilation_data.unresolved_branches.insert(self.encoder_offset - instruction_size, (*target, instruction_size));
            }
            InstructionIR::PrintStackFrame(instruction_index) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, register_call_arguments::ARG0, Register::RBP));
                self.encode_x86_instruction(X86Instruction::try_with_reg_u64(Code::Mov_r64_imm64, register_call_arguments::ARG1, function as *const _ as u64).unwrap());
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Mov_rm64_imm32, register_call_arguments::ARG2, *instruction_index as i32).unwrap());

                call_direct(
                    |instruction| self.encode_x86_instruction(instruction),
                    runtime_interface::print_stack_frame as u64
                );
            }
            InstructionIR::GarbageCollect(instruction_index) => {
                self.encode_x86_instruction(X86Instruction::with_reg_reg(Code::Mov_r64_rm64, register_call_arguments::ARG0, Register::RBP));
                self.encode_x86_instruction(X86Instruction::try_with_reg_u64(Code::Mov_r64_imm64, register_call_arguments::ARG1, function as *const _ as u64).unwrap());
                self.encode_x86_instruction(X86Instruction::try_with_reg_i32(Code::Mov_rm64_imm32, register_call_arguments::ARG2, *instruction_index as i32).unwrap());

                call_direct(
                    |instruction| self.encode_x86_instruction(instruction),
                    runtime_interface::garbage_collect as u64
                );
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

    fn compute_array_element_address(&mut self,
                                     element: &Type,
                                     reference_register: Register,
                                     index_register: Register) -> MemoryOperand {
        MemoryOperand::with_base_index_scale_displ_size(
            reference_register,
            index_register, element.size() as u32, array::LENGTH_SIZE as i32, 1
        )
    }

    fn set_jump_target(&mut self, branch_offset: usize, branch_instruction_size: usize) {
        let jump_amount = (self.encoder_offset - branch_offset) as i32 - branch_instruction_size as i32;
        let mut buffer = self.encoder.take_buffer();

        let source_offset = branch_offset as i32 + branch_instruction_size as i32 - std::mem::size_of::<i32>() as i32;
        for (i, byte) in jump_amount.to_le_bytes().iter().enumerate() {
            buffer[source_offset as usize + i] = *byte;
        }

        self.encoder.set_buffer(buffer);
    }

    pub fn encode_x86_instruction(&mut self, instruction: X86Instruction) {
        self.encode_x86_instruction_with_size(instruction);
    }

    fn encode_x86_instruction_with_size(&mut self, instruction: X86Instruction) -> usize {
        println!("\t\t{}", instruction);
        let size = self.encoder.encode(&instruction, 0).unwrap();
        self.encoder_offset += size;
        size
    }
}

pub mod register_mapping {
    use iced_x86::Register;

    use crate::compiler::calling_conventions::{float_register_call_arguments, register_call_arguments};
    use crate::compiler::FunctionCallType::Relative;
    use crate::compiler::ir::HardwareRegister;

    lazy_static! {
       static ref mapping_i64: Vec<Register> = {
           vec![
                Register::RDX,
                Register::RCX,
                Register::R8,
                Register::R9,
                Register::R10,
                Register::R11
            ]
       };

        static ref mapping_i32: Vec<Register> = {
            vec![
                Register::EDX,
                Register::ECX,
                Register::R8D,
                Register::R9D,
                Register::R10D,
                Register::R11D,
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
            HardwareRegister::IntSpill => {
                if is_64 {
                    Register::RAX
                } else {
                    Register::EAX
                }
            }
            HardwareRegister::Float(index) => {
                mapping_f32[index as usize]
            }
            HardwareRegister::FloatSpill => {
                Register::XMM0
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

pub fn push_xmm<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, Register::RAX.size() as i32).unwrap());
    encode_instruction(X86Instruction::with_mem_reg(Code::Movss_xmmm32_xmm, MemoryOperand::with_base(Register::RSP), register));
}

pub fn pop_r32<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::with_reg_mem(Code::Mov_r32_rm32, register, MemoryOperand::with_base(Register::RSP)));
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
}

pub fn pop_r64<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::with_reg_mem(Code::Mov_r64_rm64, register, MemoryOperand::with_base(Register::RSP)));
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, register.size() as i32).unwrap());
}

pub fn pop_xmm<F: FnMut(X86Instruction)>(mut encode_instruction: F, register: Register) {
    encode_instruction(X86Instruction::with_reg_mem(Code::Movss_xmm_xmmm32, register, MemoryOperand::with_base(Register::RSP)));
    encode_instruction(X86Instruction::try_with_reg_i32(Code::Add_rm64_imm32, Register::RSP, Register::RAX.size() as i32).unwrap());
}