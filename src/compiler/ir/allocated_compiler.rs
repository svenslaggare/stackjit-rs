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
use crate::optimization::register_allocation::{AllocatedRegister, RegisterAllocation};
use crate::optimization::register_allocation::linear_scan::Settings;

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
               optimization_result: &'a OptimizationResult) -> AllocatedInstructionIRCompiler<'a> {
        AllocatedInstructionIRCompiler {
            binder,
            type_storage,
            function,
            instructions: Vec::new(),
            compilation_result,
            optimization_result: optimization_result,
            register_allocation: AllocatedInstructionIRCompiler::register_allocate(compilation_result)
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

    fn register_allocate(compilation_result: &MIRCompilationResult) -> RegisterAllocation {
        let instructions = &compilation_result.instructions;
        let basic_blocks = BasicBlock::create_blocks(instructions);
        let control_flow_graph = ControlFlowGraph::new(&instructions, &basic_blocks);
        let live_intervals = liveness::compute_liveness(compilation_result, &basic_blocks, &control_flow_graph);
        register_allocation::linear_scan::allocate(
            &live_intervals,
            &Settings { num_int_registers: 2, num_float_registers: 2 }
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
                // self.print_stack_frame(instruction_index);

                let alive_registers = self.push_alive_registers(instruction_index);
                let alive_hardware_registers = alive_registers.iter().map(|(_, register)| register.clone()).collect::<Vec<_>>();

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(size);
                let (size_is_stack, size_register) = temp_registers.get_register(size);
                let size_alive = self.push_if_alive(&alive_hardware_registers, size, &size_register, size_is_stack);

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

                if size_alive {
                    self.instructions.push(InstructionIR::Pop(size_register));
                }

                self.pop_alive_registers(&alive_registers, destination_register);
            }
            InstructionMIRData::LoadElement(element, destination, array_ref, index) => {
                let alive_hardware_registers = self.register_allocation.alive_hardware_registers_at(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(array_ref);
                temp_registers.try_remove(index);

                let (array_ref_is_stack, array_ref_register) = temp_registers.get_register(array_ref);
                let (index_is_stack, index_register) = temp_registers.get_register(index);

                let array_ref_alive = self.push_if_alive(&alive_hardware_registers, array_ref, &array_ref_register, array_ref_is_stack);
                let index_alive = self.push_if_alive(&alive_hardware_registers, index, &index_register, index_is_stack);

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

                if index_alive {
                    self.instructions.push(InstructionIR::Pop(index_register));
                }

                if array_ref_alive {
                    self.instructions.push(InstructionIR::Pop(array_ref_register));
                }
            }
            InstructionMIRData::StoreElement(element, array_ref, index, value) => {
                let alive_hardware_registers = self.register_allocation.alive_hardware_registers_at(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(array_ref);
                temp_registers.try_remove(index);
                temp_registers.try_remove(value);

                let (array_ref_is_stack, array_ref_register) = temp_registers.get_register(array_ref);
                let (index_is_stack, index_register) = temp_registers.get_register(index);
                let (value_is_stack, value_register) = temp_registers.get_register(value);

                let array_ref_alive = self.push_if_alive(&alive_hardware_registers, array_ref, &array_ref_register, array_ref_is_stack);
                let index_alive = self.push_if_alive(&alive_hardware_registers, index, &index_register, index_is_stack);
                let value_alive = self.push_if_alive(&alive_hardware_registers, value, &value_register, value_is_stack);

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

                if value_alive {
                    self.instructions.push(InstructionIR::Pop(value_register));
                }

                if index_alive {
                    self.instructions.push(InstructionIR::Pop(index_register));
                }

                if array_ref_alive {
                    self.instructions.push(InstructionIR::Pop(array_ref_register));
                }
            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {
                let alive_hardware_registers = self.register_allocation.alive_hardware_registers_at(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(array_ref);

                let (array_ref_is_stack, array_ref_register) = temp_registers.get_register(array_ref);
                let array_ref_alive = self.push_if_alive(&alive_hardware_registers, array_ref, &array_ref_register, array_ref_is_stack);

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

                if array_ref_alive {
                    self.instructions.push(InstructionIR::Pop(array_ref_register));
                }
            }
            InstructionMIRData::NewObject(class_type, destination) => {
                // self.print_stack_frame(instruction_index);

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

                let alive_hardware_registers = self.register_allocation.alive_hardware_registers_at(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(class_ref);

                let (class_ref_is_stack, class_ref_register) = temp_registers.get_register(class_ref);
                let class_ref_alive = self.push_if_alive(&alive_hardware_registers, class_ref, &class_ref_register, class_ref_is_stack);

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

                if class_ref_alive {
                    self.instructions.push(InstructionIR::Pop(class_ref_register));
                }
            }
            InstructionMIRData::StoreField(class_type, field_name, class_ref, value) => {
                let class = self.type_storage.get(class_type).unwrap().class.as_ref().unwrap();
                let field = class.get_field(field_name).unwrap();

                let alive_hardware_registers = self.register_allocation.alive_hardware_registers_at(instruction_index);

                let mut temp_registers = TempRegisters::new(&self.register_allocation);
                temp_registers.try_remove(class_ref);
                temp_registers.try_remove(value);

                let (class_ref_is_stack, class_ref_register) = temp_registers.get_register(class_ref);
                let (value_is_stack, value_register) = temp_registers.get_register(value);

                let class_ref_alive = self.push_if_alive(&alive_hardware_registers, class_ref, &class_ref_register, class_ref_is_stack);
                let value_alive = self.push_if_alive(&alive_hardware_registers, value, &value_register, value_is_stack);

                if self.can_be_null(instruction_index, class_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(class_ref_register));
                }

                self.instructions.push(InstructionIR::StoreField(
                    field.type_id().clone(),
                    field.offset(),
                    class_ref_register,
                    value_register,
                ));

                if value_alive {
                    self.instructions.push(InstructionIR::Pop(value_register));
                }

                if class_ref_alive {
                    self.instructions.push(InstructionIR::Pop(class_ref_register));
                }
            }
            InstructionMIRData::GarbageCollect => {
                let alive_registers = self.push_alive_registers(instruction_index);
                self.instructions.push(InstructionIR::GarbageCollect(instruction_index));
                self.pop_alive_registers(&alive_registers, None);
            }
            InstructionMIRData::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIRData::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIRData::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                let signed = match compare_type {
                    TypeId::Int32 => {
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
                    _ => { panic!("unexpected."); }
                };

                self.instructions.push(InstructionIR::BranchCondition(
                    *condition,
                    signed,
                    *label
                ));
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
                    if !overwritten.contains(&register_mapping::get(register.clone(), true)) {
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

    fn push_alive_registers(&mut self, instruction_index: usize) -> Vec<(VirtualRegister, HardwareRegister)> {
        let alive_registers = self.register_allocation.alive_registers_at(instruction_index);
        for (virtual_register, register) in &alive_registers {
            self.instructions.push(InstructionIR::StoreFrameMemory(
                self.get_virtual_register_stack_offset(virtual_register),
                register.clone()
            ));
        }

        alive_registers
    }

    fn pop_alive_registers(&mut self,
                           alive_registers: &Vec<(VirtualRegister, HardwareRegister)>,
                           destination_register: Option<HardwareRegister>) {
        for (virtual_register, register) in alive_registers.iter().rev() {
            if let Some(destination_register) = destination_register.as_ref() {
                if destination_register != register {
                    self.instructions.push(InstructionIR::LoadFrameMemory(
                        register.clone(),
                        self.get_virtual_register_stack_offset(&virtual_register),
                    ));
                } else {
                    // The assign register will have the return value as value, so don't pop to a register.
                }
            } else {
                self.instructions.push(InstructionIR::LoadFrameMemory(
                    register.clone(),
                    self.get_virtual_register_stack_offset(&virtual_register)
                ));
            }
        }
    }

    fn push_if_alive(&mut self,
                     alive_registers: &Vec<HardwareRegister>,
                     register_ir: &RegisterMIR,
                     register: &HardwareRegister,
                     is_stack: bool) -> bool {
        let alive = if is_stack && alive_registers.contains(&register) {
            self.instructions.push(InstructionIR::Push(register.clone()));
            true
        } else {
            false
        };

        if is_stack {
            self.instructions.push(InstructionIR::LoadFrameMemory(
                register.clone(),
                self.get_register_stack_offset(register_ir)
            ));
        }

        alive
    }

    fn move_register(&mut self,
                     destination: &RegisterMIR,
                     source: &RegisterMIR) {
        let destination_allocation = self.register_allocation.get_register(destination).clone();
        let source_allocation = self.register_allocation.get_register(source).clone();

        match (destination_allocation, source_allocation) {
            (AllocatedRegister::Hardware { register: destination_register, .. }, AllocatedRegister::Hardware { register: source_register, .. }) => {
                self.instructions.push(InstructionIR::Move(destination_register, source_register));
            }
            (AllocatedRegister::Hardware { register: destination_register, .. }, AllocatedRegister::Stack { .. }) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(destination_register, self.get_register_stack_offset(source)));
            }
            (AllocatedRegister::Stack {  .. }, AllocatedRegister::Hardware { register: source_register, .. }) => {
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), source_register));
            }
            (AllocatedRegister::Stack { .. }, AllocatedRegister::Stack { .. }) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, self.get_register_stack_offset(source)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::IntSpill));
            }
        }
    }

    fn move_from_hardware_register(&mut self,
                                   destination: &RegisterMIR,
                                   source: HardwareRegister) {
        let destination_allocation = self.register_allocation.get_register(destination).clone();
        match destination_allocation.hardware_register() {
            Some(destination_register) => {
                self.instructions.push(InstructionIR::Move(destination_register, source));
            }
            None => {
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), source));
            }
        }
    }

    fn move_to_hardware_register(&mut self,
                                 destination: HardwareRegister,
                                 source: &RegisterMIR) {
        let source_allocation = self.register_allocation.get_register(source).clone();
        match source_allocation.hardware_register() {
            Some(source_register) => {
                self.instructions.push(InstructionIR::Move(destination, source_register));
            }
            None => {
                self.instructions.push(InstructionIR::LoadFrameMemory(destination, self.get_register_stack_offset(source)));
            }
        }
    }

    fn binary_operator_with_destination<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        let operand2_allocation = self.register_allocation.get_register(operand2).clone();
        let operand2_offset = self.get_register_stack_offset(operand2);

        if destination == operand1 {
            let destination_allocation = self.register_allocation.get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);

            self.binary_operator_internal(
                (destination_allocation.hardware_register(), destination_offset),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
                mem_reg
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
            self.binary_operator_internal(
                (Some(HardwareRegister::IntSpill), 0),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
                mem_reg
            );
            self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
        }
    }

    fn binary_operator<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        self.binary_operator_internal(
            (self.register_allocation.get_register(operand1).hardware_register(), self.get_register_stack_offset(operand1)),
            (self.register_allocation.get_register(operand2).hardware_register(), self.get_register_stack_offset(operand2)),
            reg_reg,
            reg_mem,
            mem_reg
        );
    }

    fn binary_operator_internal<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      operand1: (Option<HardwareRegister>, i32),
      operand2: (Option<HardwareRegister>, i32),
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        let (operand1_allocation, operand1_offset) = operand1;
        let (operand2_allocation, operand2_offset) = operand2;

        match (operand1_allocation, operand2_allocation) {
            (Some(operand1_register), Some(operand2_register)) => {
                reg_reg(&mut self.instructions, operand1_register, operand2_register);
            }
            (Some(operand1_register), None) => {
                reg_mem(&mut self.instructions, operand1_register, operand2_offset);
            }
            (None, Some(operand2_register)) => {
                mem_reg(&mut self.instructions, operand1_offset, operand2_register);
            }
            (None, None) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, operand2_offset));
                mem_reg(&mut self.instructions, operand1_offset, HardwareRegister::IntSpill);
            }
        }
    }

    fn binary_operator_with_constant_and_destination<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F2: Fn(&mut Vec<InstructionIR>, i32, i32)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: i32,
      reg_constant: F1,
      mem_constant: F2) {
        let handle = |instructions: &mut Vec<InstructionIR>, operand1: (Option<HardwareRegister>, i32), operand2: i32| {
            let (operand1_allocation, operand1_offset) = operand1;

            match operand1_allocation {
                Some(operand1_register) => {
                    reg_constant(instructions, operand1_register, operand2);
                }
                None => {
                    mem_constant(instructions, operand1_offset, operand2);
                }
            }
        };

        if destination == operand1 {
            let destination_allocation = self.register_allocation.get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);
            handle(
                &mut self.instructions,
                (destination_allocation.hardware_register(), destination_offset),
                operand2
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
            handle(
                &mut self.instructions,
                (Some(HardwareRegister::IntSpill), 0),
                operand2
            );
            self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
        }
    }

    fn binary_operator_with_destination_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2) {
        let operand2_allocation = self.register_allocation.get_register(operand2).clone();
        let operand2_offset = self.get_register_stack_offset(operand2);

        if destination == operand1 {
            let destination_allocation = self.register_allocation.get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);

            self.binary_operator_internal_f32(
                (destination_allocation.hardware_register(), destination_offset),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::FloatSpill, operand1);
            self.binary_operator_internal_f32(
                (Some(HardwareRegister::FloatSpill), 0),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
            self.move_from_hardware_register(destination, HardwareRegister::FloatSpill);
        }
    }

    fn binary_operator_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2) {
        self.binary_operator_internal_f32(
            (self.register_allocation.get_register(operand1).hardware_register(), self.get_register_stack_offset(operand1)),
            (self.register_allocation.get_register(operand2).hardware_register(), self.get_register_stack_offset(operand2)),
            reg_reg,
            reg_mem
        );
    }

    fn binary_operator_internal_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: (Option<HardwareRegister>, i32),
      operand2: (Option<HardwareRegister>, i32),
      reg_reg: F1,
      reg_mem: F2) {
        let (operand1_allocation, operand1_offset) = operand1;
        let (operand2_allocation, operand2_offset) = operand2;

        match (operand1_allocation, operand2_allocation) {
            (Some(operand1_register), Some(operand2_register)) => {
                reg_reg(&mut self.instructions, operand1_register, operand2_register);
            }
            (Some(operand1_register), None) => {
                reg_mem(&mut self.instructions, operand1_register, operand2_offset);
            }
            (None, Some(operand2_register)) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_reg(&mut self.instructions, HardwareRegister::FloatSpill, operand2_register);
                self.instructions.push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
            (None, None) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_mem(&mut self.instructions, HardwareRegister::FloatSpill, operand2_offset);
                self.instructions.push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
        }
    }

    fn get_virtual_register_stack_offset(&self, register: &VirtualRegister) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function, register.number)
    }

    fn get_register_stack_offset(&self, register: &RegisterMIR) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function, register.number)
    }
}

struct TempRegisters<'a> {
    register_allocation: &'a RegisterAllocation,
    int_registers: BTreeSet<HardwareRegister>
}

impl<'a> TempRegisters<'a> {
    pub fn new(register_allocation: &'a RegisterAllocation) -> TempRegisters<'a> {
        let mut int_registers = BTreeSet::new();
        int_registers.insert(HardwareRegister::Int(5));
        int_registers.insert(HardwareRegister::Int(4));
        int_registers.insert(HardwareRegister::Int(3));
        // int_registers.insert(HardwareRegister::Int(0));
        // int_registers.insert(HardwareRegister::Int(1));
        // int_registers.insert(HardwareRegister::Int(2));

        TempRegisters {
            register_allocation,
            int_registers
        }
    }

    pub fn try_remove(&mut self, register: &RegisterMIR) {
        if let Some(register) = self.register_allocation.get_register(register).hardware_register() {
            self.int_registers.remove(&register);
        }
    }

    pub fn get_register(&mut self, register: &RegisterMIR) -> (bool, HardwareRegister) {
        match self.register_allocation.get_register(register).hardware_register() {
            Some(register) => (false, register.clone()),
            None => {
                let register = self.int_registers.iter().rev().next().unwrap().clone();
                self.int_registers.remove(&register);
                (true, register)
            }
        }
    }
}