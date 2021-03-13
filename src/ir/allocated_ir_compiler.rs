use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use iced_x86::Register;

use crate::compiler::calling_conventions::{CallingConventions, register_call_arguments, float_register_call_arguments};
use crate::compiler::stack_layout;
use crate::engine::binder::Binder;
use crate::ir::{HardwareRegister, HardwareRegisterExplicit, InstructionIR, Variable, branches};
use crate::ir::mid::{InstructionMIR, VirtualRegister};
use crate::ir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::ir::mid::InstructionMIRData;
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::model::verifier::Verifier;
use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::analysis::liveness;
use crate::optimization::register_allocation;
use crate::optimization::register_allocation::linear_scan::Settings;
use crate::optimization::register_allocation::{RegisterAllocation, AllocatedRegister};
use crate::compiler::code_generator::register_mapping;

pub struct AllocatedInstructionIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_result: &'a MIRCompilationResult,
    register_allocation: RegisterAllocation,
    instructions: Vec<InstructionIR>
}

impl<'a> AllocatedInstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder,
               function: &'a Function,
               compilation_result: &'a MIRCompilationResult) -> AllocatedInstructionIRCompiler<'a> {
        AllocatedInstructionIRCompiler {
            binder,
            function,
            instructions: Vec::new(),
            compilation_result,
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
        let branch_label_mapping = branches::create_label_mapping(&instructions);
        let control_flow_graph = ControlFlowGraph::new(&instructions, &basic_blocks, &branch_label_mapping);
        let live_intervals = liveness::compute_liveness(instructions, &basic_blocks, &control_flow_graph);
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
            // self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::IntSpill));
            let mut int_initialized = false;
            let mut float_initialized = false;

            for register in &self.compilation_result.need_zero_initialize_registers {
                match register.value_type {
                    Type::Float32 => {
                        if !float_initialized {
                            self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::FloatSpill));
                            float_initialized = true;
                        }

                        match self.register_allocation.get_register(register) {
                            AllocatedRegister::Hardware { register, .. } => {
                                self.instructions.push(InstructionIR::Move(*register, HardwareRegister::FloatSpill));
                            }
                            AllocatedRegister::Stack { .. } => {
                                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(register), HardwareRegister::FloatSpill));
                            }
                        }
                    }
                    _ => {
                        if !int_initialized {
                            self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::IntSpill));
                            int_initialized = true;
                        }

                        match self.register_allocation.get_register(register) {
                            AllocatedRegister::Hardware { register, .. } => {
                                self.instructions.push(InstructionIR::Move(*register, HardwareRegister::IntSpill));
                            }
                            AllocatedRegister::Stack { .. } => {
                                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(register), HardwareRegister::IntSpill));
                            }
                        }
                    }
                }

                // match self.register_allocation.get_register(register) {
                //     AllocatedRegister::Hardware { register, .. } => {
                //         self.instructions.push(InstructionIR::Move(*register, HardwareRegister::IntSpill));
                //     }
                //     AllocatedRegister::Stack { .. } => {
                //         self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(register), HardwareRegister::IntSpill));
                //     }
                // }
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &InstructionMIR) {
        self.instructions.push(InstructionIR::Marker(instruction.index));

        match &instruction.data {
            InstructionMIRData::LoadInt32(destination, value) => {
                match self.register_allocation.get_register(destination) {
                    AllocatedRegister::Hardware { register, .. } => {
                        self.instructions.push(InstructionIR::MoveInt32ToRegister(*register, *value));
                    }
                    AllocatedRegister::Stack { .. } => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), *value));
                    }
                }
            }
            InstructionMIRData::LoadFloat32(destination, value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };

                match self.register_allocation.get_register(destination) {
                    AllocatedRegister::Hardware { register, .. } => {
                        self.instructions.push(InstructionIR::PushInt32(value));
                        self.instructions.push(InstructionIR::Pop(*register));
                    }
                    AllocatedRegister::Stack { .. } => {
                        self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), value));
                    }
                }
            }
            InstructionMIRData::Move(destination, source) => {
                self.move_register(destination, source);
            }
            InstructionMIRData::AddInt32(destination, operand1, operand2) => {
                self.binary_operator(
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

                if destination != operand1 {
                    self.move_register(destination, operand1);
                }
            }
            InstructionMIRData::SubInt32(destination, operand1, operand2) => {
                self.binary_operator(
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

                if destination != operand1 {
                    self.move_register(destination, operand1);
                }
            }
            InstructionMIRData::AddFloat32(destination, operand1, operand2) => {
                self.binary_operator_f32(
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::AddFloat32FromFrameMemory(op1, op2));
                    }
                );

                if destination != operand1 {
                    self.move_register(destination, operand1);
                }
            }
            InstructionMIRData::SubFloat32(destination, operand1, operand2) => {
                self.binary_operator_f32(
                    operand1,
                    operand2,
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubFloat32(op1, op2));
                    },
                    |instructions, op1, op2| {
                        instructions.push(InstructionIR::SubFloat32FromFrameMemory(op1, op2));
                    }
                );

                if destination != operand1 {
                    self.move_register(destination, operand1);
                }
            }
            InstructionMIRData::Return(source) => {
                if let Some(source) = source {
                    CallingConventions::new().make_return_value(
                        self.function,
                        &match self.register_allocation.get_register(source) {
                            AllocatedRegister::Hardware { register, .. } => Variable::Register(register.clone()),
                            AllocatedRegister::Stack { .. } => Variable::FrameMemory(self.get_register_stack_offset(source))
                        },
                        &mut self.instructions
                    );
                }

                self.instructions.push(InstructionIR::Return);
            }
            InstructionMIRData::Call(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let alive_registers = self.register_allocation.alive_registers_at(instruction_index);

                let alive_registers_mapping = HashMap::<HardwareRegister, usize>::from_iter(
                    alive_registers
                        .iter().enumerate()
                        .map(|(index, register)| (register.clone(), index))
                );

                for register in &alive_registers{
                    self.instructions.push(InstructionIR::Push(register.clone()));
                }

                let arguments_source = self.get_call_argument_sources(
                    &alive_registers_mapping,
                    func_to_call,
                    arguments
                );

                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source, alive_registers.len()));

                let return_register = if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &match self.register_allocation.get_register(return_value) {
                            AllocatedRegister::Hardware { register, .. } => Variable::Register(register.clone()),
                            AllocatedRegister::Stack { .. } => Variable::FrameMemory(self.get_register_stack_offset(return_value))
                        },
                        func_to_call,
                        &mut self.instructions
                    );

                    self.register_allocation.get_register(return_value).hardware_register()
                } else {
                    None
                };

                for register in alive_registers.iter().rev() {
                    if let Some(return_register) = return_register.as_ref() {
                        if return_register != register {
                            self.instructions.push(InstructionIR::Pop(register.clone()));
                        } else {
                            // The assign register will have the return value as value, so don't pop to a register.
                            self.instructions.push(InstructionIR::PopEmpty);
                        }
                    } else {
                        self.instructions.push(InstructionIR::Pop(register.clone()));
                    }
                }
            }
            InstructionMIRData::LoadArgument(argument_index, destination) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                match self.register_allocation.get_register(destination) {
                    AllocatedRegister::Hardware { register, .. } => {
                        self.instructions.push(InstructionIR::LoadFrameMemory(register.clone(), argument_offset));
                    }
                    AllocatedRegister::Stack { .. } => {
                        self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, argument_offset));
                        self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::IntSpill));
                    }
                }
            }
            InstructionMIRData::LoadNull(destination) => {

            }
            InstructionMIRData::NewArray(element, destination, size) => {

            }
            InstructionMIRData::LoadElement(element, destination, array_ref, index) => {

            }
            InstructionMIRData::StoreElement(element, array_ref, index, value) => {

            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {

            }
            InstructionMIRData::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIRData::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIRData::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                let signed = match compare_type {
                    Type::Int32 => {
                        self.binary_operator(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(Type::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(Type::Int32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareToFrameMemory(Type::Int32, op1, op2));
                            }
                        );
                        true
                    }
                    Type::Float32 => {
                        self.binary_operator_f32(
                            operand1,
                            operand2,
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::Compare(Type::Float32, op1, op2));
                            },
                            |instructions, op1, op2| {
                                instructions.push(InstructionIR::CompareFromFrameMemory(Type::Float32, op1, op2));
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

    fn get_call_argument_sources(&self,
                                 alive_registers_mapping: &HashMap<HardwareRegister, usize>,
                                 func_to_call: &FunctionDefinition,
                                 arguments: &Vec<VirtualRegister>) -> Vec<Variable> {
        // arguments
        //     .iter()
        //     .map(|argument| {
        //         match self.register_allocation.get_register(argument) {
        //             AllocatedRegister::Hardware { register, .. } => {
        //                 let argument_stack_index = alive_registers_mapping[register] as u32;
        //                 let stack_offset = stack_layout::stack_value_offset(self.function, self.compilation_result, argument_stack_index);
        //                 Variable::FrameMemory(stack_offset)
        //             },
        //             AllocatedRegister::Stack { .. } => {
        //                 Variable::FrameMemory(self.get_register_stack_offset(argument))
        //             }
        //         }
        //     })
        //     .collect::<Vec<_>>()

        let mut variables = Vec::new();

        let mut overwritten = HashSet::new();
        for (index, argument) in arguments.iter().enumerate().rev() {
            match self.register_allocation.get_register(argument) {
                AllocatedRegister::Hardware { register, .. } => {
                    if !overwritten.contains(&register_mapping::get(register.clone(), true)) {
                        variables.push(Variable::Register(register.clone()));
                    } else {
                        variables.push(Variable::FrameMemory(
                            stack_layout::stack_value_offset(
                                self.function,
                                self.compilation_result,
                                alive_registers_mapping[register] as u32
                            )
                        ));
                    }
                }
                AllocatedRegister::Stack { .. } => {
                    variables.push(Variable::FrameMemory(self.get_register_stack_offset(argument)));
                }
            }

            match argument.value_type {
                Type::Float32 => {
                    let relative_index = float_register_call_arguments::get_relative_index(func_to_call.parameters(), index);
                    if relative_index < float_register_call_arguments::NUM_ARGUMENTS {
                        overwritten.insert(float_register_call_arguments::get_argument(relative_index));
                    }
                }
                _ => {
                    let relative_index = register_call_arguments::get_relative_index(func_to_call.parameters(), index);
                    if relative_index < register_call_arguments::NUM_ARGUMENTS {
                        overwritten.insert(register_call_arguments::get_argument(relative_index));
                    }
                }
            }
        }

        variables.reverse();
        variables
    }

    fn move_register(&mut self,
                     destination: &VirtualRegister,
                     source: &VirtualRegister,) {
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

    fn binary_operator<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      operand1: &VirtualRegister,
      operand2: &VirtualRegister,
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        let operand1_allocation = self.register_allocation.get_register(operand1).clone();
        let operand2_allocation = self.register_allocation.get_register(operand2).clone();

        use AllocatedRegister::{Hardware, Stack};
        match (operand1_allocation, operand2_allocation) {
            (Hardware { register: operand1_register, .. },  Hardware { register: operand2_register, .. }) => {
                reg_reg(&mut self.instructions, operand1_register, operand2_register);
            }
            (Hardware { register: operand1_register, .. }, Stack { .. }) => {
                let operand2_offset = self.get_register_stack_offset(operand2);
                reg_mem(&mut self.instructions, operand1_register, operand2_offset);
            }
            (Stack { ..}, Hardware { register: operand2_register, .. }) => {
                let operand1_offset = self.get_register_stack_offset(operand1);
                mem_reg(&mut self.instructions, operand1_offset, operand2_register);
            }
            (Stack { .. }, Stack { .. }) => {
                let operand1_offset = self.get_register_stack_offset(operand1);
                let operand2_offset = self.get_register_stack_offset(operand2);
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, operand2_offset));
                mem_reg(&mut self.instructions, operand1_offset, HardwareRegister::IntSpill);
            }
        }
    }

    fn binary_operator_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: &VirtualRegister,
      operand2: &VirtualRegister,
      reg_reg: F1,
      reg_mem: F2) {
        let operand1_allocation = self.register_allocation.get_register(operand1).clone();
        let operand2_allocation = self.register_allocation.get_register(operand2).clone();

        use AllocatedRegister::{Hardware, Stack};
        match (operand1_allocation, operand2_allocation) {
            (Hardware { register: operand1_register, .. },  Hardware { register: operand2_register, .. }) => {
                reg_reg(&mut self.instructions, operand1_register, operand2_register);
            }
            (Hardware { register: operand1_register, .. }, Stack { .. }) => {
                let operand2_offset = self.get_register_stack_offset(operand2);
                reg_mem(&mut self.instructions, operand1_register, operand2_offset);
            }
            (Stack { ..}, Hardware { register: operand2_register, .. }) => {
                let operand1_offset = self.get_register_stack_offset(operand1);
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_reg(&mut self.instructions, HardwareRegister::FloatSpill, operand2_register);
                self.instructions.push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
            (Stack { .. }, Stack { .. }) => {
                let operand1_offset = self.get_register_stack_offset(operand1);
                let operand2_offset = self.get_register_stack_offset(operand2);
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_mem(&mut self.instructions, HardwareRegister::FloatSpill, operand2_offset);
                self.instructions.push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
        }
    }

    fn get_register_stack_offset(&self, register: &VirtualRegister) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function, register.number)
    }
}