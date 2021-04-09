use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter::FromIterator;

use crate::analysis::{OptimizationResult, liveness, VirtualRegister};
use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::compiler::calling_conventions::{CallingConventions, float_register_call_arguments, get_call_register, register_call_arguments};
use crate::compiler::code_generator::register_mapping;
use crate::compiler::ir::{HardwareRegister, HardwareRegisterExplicit, InstructionIR, Variable};
use crate::compiler::stack_layout;
use crate::model::binder::Binder;
use crate::mir::{branches, InstructionMIR, RegisterMIR};
use crate::mir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::mir::InstructionMIRData;
use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{TypeId, TypeStorage};
use crate::model::verifier::Verifier;
use crate::optimization::register_allocation;
use crate::optimization::register_allocation::{AllocatedRegister, RegisterAllocation, RegisterAllocationSettings};
use crate::compiler::code_generator::register_mapping::DataSize;
use crate::compiler::ir::helpers::{AllocatedCompilerHelpers, TempRegisters};
use iced_x86::Register;

pub struct AllocatedInstructionIRCompiler<'a> {
    binder: &'a Binder,
    type_storage: &'a TypeStorage,
    function: &'a Function,
    compilation_result: &'a MIRCompilationResult,
    optimization_result: &'a OptimizationResult,
    register_allocation: RegisterAllocation,
    instructions: Vec<InstructionIR>
}

impl<'a> AllocatedInstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder,
               type_storage: &'a TypeStorage,
               function: &'a Function,
               compilation_result: &'a MIRCompilationResult,
               optimization_result: &'a OptimizationResult,
               register_allocation_settings: &RegisterAllocationSettings) -> AllocatedInstructionIRCompiler<'a> {
        AllocatedInstructionIRCompiler {
            binder,
            type_storage,
            function,
            instructions: Vec::new(),
            compilation_result,
            optimization_result,
            register_allocation: AllocatedInstructionIRCompiler::register_allocate(register_allocation_settings, compilation_result)
        }
    }

    pub fn done(self) -> Vec<InstructionIR> {
        self.instructions
    }

    pub fn compile(&mut self) {
        self.compile_initialize_function();

        for (instruction_index, instruction) in self.compilation_result.instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn register_allocate(register_allocation_settings: &RegisterAllocationSettings,
                         compilation_result: &MIRCompilationResult) -> RegisterAllocation {
        let instructions = &compilation_result.instructions;
        let basic_blocks = BasicBlock::create_blocks(instructions);
        let control_flow_graph = ControlFlowGraph::new(&instructions, &basic_blocks);
        let live_intervals = liveness::compute(compilation_result, &basic_blocks, &control_flow_graph);
        register_allocation::linear_scan::allocate(
            &live_intervals,
            register_allocation_settings
        )
    }

    fn compile_initialize_function(&mut self) {
        self.instructions.push(InstructionIR::InitializeFunction);

        let stack_size = stack_layout::stack_size(self.function, self.compilation_result);
        if stack_size > 0 {
            self.instructions.push(InstructionIR::SubFromStackPointer(stack_size));
        }

        CallingConventions::new().move_arguments_to_stack(self.function, &mut self.instructions);

        if !self.compilation_result.need_zero_initialize_registers.is_empty() {
            let mut int_initialized = false;
            let mut float_initialized = false;

            for register in &self.compilation_result.need_zero_initialize_registers {
                if self.register_allocation.is_used(register) {
                    let allocated_register = self.register_allocation.get_register(register).hardware_register();

                    match register.value_type {
                        TypeId::Float32 => {
                            if !float_initialized {
                                self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::FloatSpill));
                                float_initialized = true;
                            }

                            if let Some(register) = allocated_register{
                                self.instructions.push(InstructionIR::Move(register, HardwareRegister::FloatSpill));
                            } else {
                                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(register), HardwareRegister::FloatSpill));
                            }
                        }
                        _ => {
                            if !int_initialized {
                                self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::IntSpill));
                                int_initialized = true;
                            }

                            if let Some(register) = allocated_register {
                                self.instructions.push(InstructionIR::Move(register, HardwareRegister::IntSpill));
                            } else {
                                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(register), HardwareRegister::IntSpill));
                            }
                        }
                    }
                }
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &InstructionMIR) {
        self.instructions.push(InstructionIR::Marker(instruction.index, instruction_index));

        match &instruction.data {
            InstructionMIRData::LoadInt32(destination, value) => {
                match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::MoveInt32ToRegister(register, *value));
                    }
                    None => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), *value));
                    }
                }
            }
            InstructionMIRData::LoadFloat32(destination, value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };

                match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::PushInt32(value));
                        self.instructions.push(InstructionIR::Pop(register));
                    }
                    None => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), value));
                    }
                }
            }
            InstructionMIRData::LoadBool(destination, value) => {
                let value = if *value {1} else {0};
                match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::MoveInt32ToRegister(register, value));
                    }
                    None => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), value));
                    }
                }
            }
            InstructionMIRData::Move(destination, source) => {
                self.move_register(destination, source);
            }
            InstructionMIRData::AddInt32(destination, operand1, operand2) => {
                self.binary_operator_with_destination(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddInt32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddInt32FromFrameMemory(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddInt32ToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::AddInt32Constant(destination, operand1, operand2) => {
                self.binary_operator_with_constant_and_destination(
                    destination,
                    operand1,
                    *operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddInt32Constant(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddInt32ConstantToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::SubInt32(destination, operand1, operand2) => {
                self.binary_operator_with_destination(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubInt32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubInt32FromFrameMemory(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubInt32ToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::SubInt32Constant(destination, operand1, operand2) => {
                self.binary_operator_with_constant_and_destination(
                    destination,
                    operand1,
                    *operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubInt32Constant(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubInt32ConstantToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::MultiplyInt32(destination, operand1, operand2) => {
                self.binary_operator_no_memory_store_with_destination(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::MultiplyInt32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::MultiplyInt32FromFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::DivideInt32(destination, operand1, operand2) => {
                let alive_registers = self.push_alive_registers(instruction_index);

                self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
                self.move_to_hardware_register(HardwareRegister::Int(5), operand2);
                self.instructions.push(InstructionIR::DivideInt32(HardwareRegister::IntSpill, HardwareRegister::Int(5)));

                self.pop_alive_registers(&alive_registers, Some(HardwareRegister::IntSpill));
                self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
            }
            InstructionMIRData::AndBool(destination, operand1, operand2) => {
                self.binary_operator_with_destination(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AndInt32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AndInt32FromFrameMemory(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AndInt32ToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::AndBoolConstant(destination, operand1, operand2) => {
                self.binary_operator_with_constant_and_destination(
                    destination,
                    operand1,
                    if *operand2 {1} else {0},
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AndInt32Constant(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AndInt32ConstantToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::OrBool(destination, operand1, operand2) => {
                self.binary_operator_with_destination(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::OrInt32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::OrInt32FromFrameMemory(op1, op2));
                    },
                    |instructions: &mut Vec<InstructionIR>, op1, op2| {
                        instructions.push(InstructionIR::OrInt32ToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::OrBoolConstant(destination, operand1, operand2) => {
                self.binary_operator_with_constant_and_destination(
                    destination,
                    operand1,
                    if *operand2 {1} else {0},
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::OrInt32Constant(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::OrInt32ConstantToFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::NotBool(destination, operand) => {
                if destination == operand {
                    match self.register_allocation().get_register(destination).hardware_register() {
                        Some(register) => {
                            self.instructions.push(InstructionIR::NotInt32(register));
                        }
                        None => {
                            self.instructions.push(InstructionIR::NotInt32FrameMemory(self.get_register_stack_offset(destination)));
                        }
                    }
                } else {
                    self.move_to_hardware_register(HardwareRegister::IntSpill, operand);
                    self.instructions.push(InstructionIR::NotInt32(HardwareRegister::IntSpill));
                    self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
                }
            }
            InstructionMIRData::AddFloat32(destination, operand1, operand2) => {
                self.binary_operator_with_destination_f32(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddFloat32FromFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::SubFloat32(destination, operand1, operand2) => {
                self.binary_operator_with_destination_f32(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubFloat32FromFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::MultiplyFloat32(destination, operand1, operand2) => {
                self.binary_operator_with_destination_f32(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::MultiplyFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::MultiplyFloat32FromFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::DivideFloat32(destination, operand1, operand2) => {
                self.binary_operator_with_destination_f32(
                    destination,
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::DivideFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::DivideFloat32FromFrameMemory(op1, op2));
                    }
                );
            }
            InstructionMIRData::Return(source) => {
                if let Some(source) = source {
                    CallingConventions::new().make_return_value(
                        self.function,
                        &match self.register_allocation.get_register(source).hardware_register() {
                            Some(register) => Variable::Register(register.clone()),
                            None => Variable::FrameMemory(self.get_register_stack_offset(source))
                        },
                        &mut self.instructions
                    );
                }

                self.instructions.push(InstructionIR::Return);
            }
            InstructionMIRData::Call(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let alive_registers = self.push_alive_registers(instruction_index);

                let arguments_source = self.get_call_argument_sources(func_to_call, arguments);
                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source, 0));

                let return_register = if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &match self.register_allocation.get_register(return_value).hardware_register() {
                            Some(register) => Variable::Register(register.clone()),
                            None => Variable::FrameMemory(self.get_register_stack_offset(return_value))
                        },
                        func_to_call,
                        &mut self.instructions
                    );

                    self.register_allocation.get_register(return_value).hardware_register()
                } else {
                    None
                };

                self.pop_alive_registers(&alive_registers, return_register);
            }
            InstructionMIRData::LoadArgument(argument_index, destination) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                if let Some(register) = self.register_allocation.get_register(destination).hardware_register() {
                    self.instructions.push(InstructionIR::LoadFrameMemory(register, argument_offset));
                } else {
                    self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, argument_offset));
                    self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::IntSpill));
                }
            }
            InstructionMIRData::LoadNull(destination) => {
                match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::MoveInt32ToRegister(register, 0));
                    }
                    None => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), 0));
                    }
                }
            }
            InstructionMIRData::NewArray(element, destination, size) => {
                let alive_registers = self.push_alive_registers(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, size);

                let (size_register, _, size_alive) = temp_registers.get_with_status(&self.register_allocation, self.function, &mut self.instructions, size);

                self.instructions.push(InstructionIR::NewArray(element.clone(), size_register, if size_alive {1} else {0}));

                let destination_register = match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::MoveExplicitToImplicit(
                            register.clone(),
                            HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                        ));

                        Some(register)
                    }
                    None => {
                        self.instructions.push(InstructionIR::StoreFrameMemoryExplicit(
                            self.get_register_stack_offset(destination),
                            HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                        ));

                        None
                    }
                };

                temp_registers.done(&mut self.instructions);

                self.pop_alive_registers(&alive_registers, destination_register);
            }
            InstructionMIRData::LoadElement(element, destination, array_ref, index) => {
                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, array_ref);
                temp_registers.try_remove(&self.register_allocation, index);

                let array_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, array_ref);
                let index_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, index);

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(array_ref_register));
                }

                self.instructions.push(InstructionIR::ArrayBoundsCheck(array_ref_register, index_register));

                let return_value = match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => register,
                    None => {
                        match element {
                            TypeId::Float32 => HardwareRegister::FloatSpill,
                            _ => HardwareRegister::IntSpill
                        }
                    }
                };

                self.instructions.push(InstructionIR::LoadElement(
                    element.clone(),
                    return_value,
                    array_ref_register,
                    index_register
                ));

                if self.register_allocation.get_register(destination).is_stack() {
                    self.instructions.push(InstructionIR::StoreFrameMemory(
                        self.get_register_stack_offset(destination),
                        return_value
                    ));
                }

                temp_registers.done(&mut self.instructions);
            }
            InstructionMIRData::StoreElement(element, array_ref, index, value) => {
                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, array_ref);
                temp_registers.try_remove(&self.register_allocation, index);
                temp_registers.try_remove(&self.register_allocation, value);

                let array_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, array_ref);
                let index_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, index);
                let value_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, value);

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(array_ref_register));
                }

                self.instructions.push(InstructionIR::ArrayBoundsCheck(array_ref_register, index_register));

                self.instructions.push(InstructionIR::StoreElement(
                    element.clone(),
                    array_ref_register,
                    index_register,
                    value_register
                ));

                temp_registers.done(&mut self.instructions);
            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {
                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, array_ref);

                let array_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, array_ref);

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(array_ref_register));
                }

                let return_value = match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => register,
                    None => HardwareRegister::IntSpill
                };

                self.instructions.push(InstructionIR::LoadArrayLength(return_value, array_ref_register));

                if self.register_allocation.get_register(destination).is_stack() {
                    self.instructions.push(InstructionIR::StoreFrameMemory(
                        self.get_register_stack_offset(destination),
                        return_value
                    ));
                }

                temp_registers.done(&mut self.instructions);
            }
            InstructionMIRData::NewObject(class_type, destination) => {
                let alive_registers = self.push_alive_registers(instruction_index);

                self.instructions.push(InstructionIR::NewObject(class_type.clone()));

                let destination_register = match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => {
                        self.instructions.push(InstructionIR::MoveExplicitToImplicit(
                            register.clone(),
                            HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                        ));

                        Some(register)
                    }
                    None => {
                        self.instructions.push(InstructionIR::StoreFrameMemoryExplicit(
                            self.get_register_stack_offset(destination),
                            HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                        ));

                        None
                    }
                };

                self.pop_alive_registers(&alive_registers, destination_register);
            }
            InstructionMIRData::LoadField(class_type, field_name, destination, class_ref) => {
                let class = self.type_storage.get(class_type).unwrap().class.as_ref().unwrap();
                let field = class.get_field(field_name).unwrap();

                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, class_ref);

                let class_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, class_ref);

                if self.can_be_null(instruction_index, class_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(class_ref_register));
                }

                let return_value = match self.register_allocation.get_register(destination).hardware_register() {
                    Some(register) => register,
                    None => {
                        match field.type_id() {
                            TypeId::Float32 => HardwareRegister::FloatSpill,
                            _ => HardwareRegister::IntSpill
                        }
                    }
                };

                self.instructions.push(InstructionIR::LoadField(
                    field.type_id().clone(),
                    field.offset(),
                    return_value,
                    class_ref_register,
                ));


                if self.register_allocation.get_register(destination).is_stack() {
                    self.instructions.push(InstructionIR::StoreFrameMemory(
                        self.get_register_stack_offset(destination),
                        return_value
                    ));
                }

                temp_registers.done(&mut self.instructions);
            }
            InstructionMIRData::StoreField(class_type, field_name, class_ref, value) => {
                let class = self.type_storage.get(class_type).unwrap().class.as_ref().unwrap();
                let field = class.get_field(field_name).unwrap();

                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, class_ref);
                temp_registers.try_remove(&self.register_allocation, value);

                let class_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, class_ref);
                let value_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, value);

                if self.can_be_null(instruction_index, class_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(class_ref_register));
                }

                self.instructions.push(InstructionIR::StoreField(
                    field.type_id().clone(),
                    field.offset(),
                    class_ref_register,
                    value_register,
                ));

                temp_registers.done(&mut self.instructions);
            }
            InstructionMIRData::CallInstance(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let alive_registers = self.push_alive_registers(instruction_index);

                let class_ref = &arguments[0];
                if self.can_be_null(instruction_index, class_ref) {
                    let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                    temp_registers.try_remove(&self.register_allocation, class_ref);

                    let class_ref_register = temp_registers.get(&self.register_allocation, self.function, &mut self.instructions, class_ref);

                    self.move_to_hardware_register(class_ref_register, class_ref);
                    self.instructions.push(InstructionIR::NullReferenceCheck(class_ref_register));

                    temp_registers.done(&mut self.instructions);
                }

                let arguments_source = self.get_call_argument_sources(func_to_call, arguments);
                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source, 0));

                let return_register = if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &match self.register_allocation.get_register(return_value).hardware_register() {
                            Some(register) => Variable::Register(register.clone()),
                            None => Variable::FrameMemory(self.get_register_stack_offset(return_value))
                        },
                        func_to_call,
                        &mut self.instructions
                    );

                    self.register_allocation.get_register(return_value).hardware_register()
                } else {
                    None
                };

                self.pop_alive_registers(&alive_registers, return_register);
            }
            InstructionMIRData::GarbageCollect => {
                let alive_registers = self.push_alive_registers(instruction_index);
                self.instructions.push(InstructionIR::GarbageCollect(instruction_index));
                self.pop_alive_registers(&alive_registers, None);
            }
            InstructionMIRData::PrintStackFrame => {
                self.print_stack_frame(instruction_index);
            }
            InstructionMIRData::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIRData::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIRData::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                let signed = match compare_type {
                    TypeId::Void => {
                        panic!("Can't compare void.");
                    }
                    TypeId::Float32 => {
                        self.binary_operator_f32(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(TypeId::Float32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(TypeId::Float32, op1, op2));
                            }
                        );

                        false
                    }
                    _ => {
                        self.binary_operator(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(TypeId::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(TypeId::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareToFrameMemory(TypeId::Int32, op1, op2));
                            }
                        );
                        true
                    }
                };

                self.instructions.push(InstructionIR::BranchCondition(
                    *condition,
                    signed,
                    *label
                ));
            }
            InstructionMIRData::Compare(condition, compare_type, destination, operand1, operand2) => {
                let mut temp_registers = TempRegisters::new(&self.register_allocation, instruction_index);
                temp_registers.try_remove(&self.register_allocation, operand1);
                temp_registers.try_remove(&self.register_allocation, operand2);

                let (destination_register, destination_is_stack, destination_alive) = temp_registers.get_with_status(
                    &self.register_allocation,
                    self.function,
                    &mut self.instructions,
                    destination
                );

                let signed = match compare_type {
                    TypeId::Void => {
                        panic!("Can't compare void.");
                    }
                    TypeId::Float32 => {
                        self.binary_operator_f32(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(TypeId::Float32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(TypeId::Float32, op1, op2));
                            }
                        );

                        false
                    }
                    _ => {
                        self.binary_operator(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(TypeId::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(TypeId::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareToFrameMemory(TypeId::Int32, op1, op2));
                            }
                        );
                        true
                    }
                };

                self.instructions.push(InstructionIR::CompareResult(
                    *condition,
                    signed,
                    destination_register
                ));

                if destination_is_stack {
                    self.move_from_hardware_register(destination, destination_register);
                }

                if destination_alive {
                    self.instructions.push(InstructionIR::Pop(destination_register));
                }

                temp_registers.clear();
            }
        }
    }

    fn print_stack_frame(&mut self, instruction_index: usize) {
        let alive_registers = self.push_alive_registers(instruction_index);
        self.instructions.push(InstructionIR::PrintStackFrame(instruction_index));
        self.pop_alive_registers(&alive_registers, None);
    }

    fn get_call_argument_sources(&self, func_to_call: &FunctionDeclaration, arguments: &Vec<RegisterMIR>) -> Vec<Variable> {
        let mut variables = Vec::new();

        let mut overwritten = HashSet::new();
        for (index, argument) in arguments.iter().enumerate().rev() {
            match self.register_allocation.get_register(argument) {
                AllocatedRegister::Hardware { register, .. } => {
                    // We might overwrite the register value when doing moves to the register arguments,
                    // so in that case, use cached version of the register on the stack that is created as part of the save register operation
                    if !overwritten.contains(&register_mapping::get(register.clone(), DataSize::Bytes8)) {
                        variables.push(Variable::Register(register.clone()));
                    } else {
                        variables.push(Variable::FrameMemory(self.get_register_stack_offset(argument)));
                    }
                }
                AllocatedRegister::Stack { .. } => {
                    variables.push(Variable::FrameMemory(self.get_register_stack_offset(argument)));
                }
            }

            if let Some(call_register) = get_call_register(func_to_call, index, &argument.value_type) {
                overwritten.insert(call_register);
            }
        }

        variables.reverse();
        variables
    }

    fn can_be_null(&self, instruction_index: usize, register: &RegisterMIR) -> bool {
        assert!(register.value_type.is_reference());
        self.optimization_result.instructions_register_null_status[instruction_index].get(register).cloned().unwrap_or(true)
    }
}

impl<'a> AllocatedCompilerHelpers for AllocatedInstructionIRCompiler<'a> {
    fn function(&self) -> &Function {
        &self.function
    }

    fn register_allocation(&self) -> &RegisterAllocation {
        &self.register_allocation
    }

    fn instructions(&mut self) -> &mut Vec<InstructionIR> {
        &mut self.instructions
    }
}
