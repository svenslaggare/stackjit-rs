use std::collections::{HashMap, HashSet};

use crate::model::function::Function;
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::compiler::FunctionCompilationData;
use crate::compiler::stack_layout;
use crate::compiler::calling_conventions::{CallingConventions};
use crate::engine::binder::Binder;
use crate::ir::{InstructionIR, HardwareRegister, BranchLabel, JumpCondition};
use crate::model::instruction;

pub struct InstructionIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_data: &'a mut FunctionCompilationData,
    instructions: Vec<InstructionIR>,
    branch_targets: HashSet<instruction::BranchTarget>,
    branch_labels: HashMap<instruction::BranchTarget, BranchLabel>,
    next_branch_label: BranchLabel
}

impl<'a> InstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function, compilation_data: &'a mut FunctionCompilationData) -> InstructionIRCompiler<'a> {
        InstructionIRCompiler {
            binder,
            function,
            compilation_data,
            instructions: Vec::new(),
            branch_targets: HashSet::new(),
            branch_labels: HashMap::new(),
            next_branch_label: 0
        }
    }

    pub fn compile(&mut self, instructions: &Vec<Instruction>) {
        self.compile_initialize_function();
        self.define_branch_labels(instructions);

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn define_branch_labels(&mut self, instructions: &Vec<Instruction>) {
        for instruction in instructions {
            if let Some(target) = instruction.branch_target() {
                self.branch_targets.insert(target);

                if !self.branch_labels.contains_key(&target) {
                    let label = self.next_branch_label;
                    self.next_branch_label += 1;
                    self.branch_labels.insert(target, label);
                }
            }
        }
    }

    fn compile_initialize_function(&mut self) {
        self.instructions.push(InstructionIR::InitializeFunction);

        //Calculate the size of the stack aligned to 16 bytes
        let needed_stack_size = stack_layout::stack_size(self.function);
        let stack_size = ((needed_stack_size + 15) / 16) * 16;

        if stack_size > 0 {
            self.instructions.push(InstructionIR::SubFromStackPointer(stack_size));
        }

        CallingConventions::new().move_arguments_to_stack(self.function, &mut self.instructions);

        // Zero locals
        let num_locals = self.function.locals().len();
        if num_locals > 0 {
            self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::Int(0)));
            for local_index in 0..(num_locals as u32) {
                let local_offset = stack_layout::local_stack_offset(self.function, local_index);
                self.instructions.push(InstructionIR::StoreMemory(local_offset, HardwareRegister::Int(0)));
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &Instruction) {
        self.instructions.push(InstructionIR::Marker(instruction_index));

        let branch_target = instruction_index as instruction::BranchTarget;
        if self.branch_targets.contains(&branch_target) {
            self.instructions.push(InstructionIR::BranchLabel(self.branch_labels[&branch_target]));
        }

        match instruction {
            Instruction::LoadInt32(value) => {
                self.instructions.push(InstructionIR::LoadInt32(*value));
            }
            Instruction::LoadFloat32(value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };
                self.instructions.push(InstructionIR::LoadInt32(value));
            }
            Instruction::LoadLocal(index) => {
                let local_offset = stack_layout::local_stack_offset(self.function, *index);
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), local_offset));
                self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Int(0)));
            }
            Instruction::StoreLocal(index) => {
                let local_offset = stack_layout::local_stack_offset(self.function, *index);
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemory(local_offset, HardwareRegister::Int(0)));
            }
            Instruction::Add => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::AddInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Int(0)));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::AddFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Float(0)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Sub => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::SubInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Int(0)));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::SubFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Float(0)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Call(signature) => {
                self.instructions.push(InstructionIR::Call(signature.clone()));
                let func_to_call = self.binder.get(signature).unwrap();
                CallingConventions::new().handle_return_value(self.function, &mut self.instructions, func_to_call);
            }
            Instruction::LoadArgument(argument_index) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Int(0)));
            }
            Instruction::Return => {
                CallingConventions::new().make_return_value(self.function, &mut self.instructions);
                self.instructions.push(InstructionIR::Return);
            }
            Instruction::NewArray(element) => {
                self.instructions.push(InstructionIR::NewArray(element.clone()));
            }
            Instruction::LoadElement(element) => {
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1))); // The index of the element
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0))); // The array reference
                self.instructions.push(InstructionIR::LoadElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1)));
            }
            Instruction::StoreElement(element) => {
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(2))); // The value to store
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1))); // The index of the element
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0))); // The array reference
                self.instructions.push(InstructionIR::StoreElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1), HardwareRegister::Int(2)));
            }
            Instruction::Branch(target) => {
                self.instructions.push(InstructionIR::Branch(self.branch_labels[target]));
            }
            Instruction::BranchEqual(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::Equal,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::Equal,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::BranchNotEqual(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::NotEqual,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::NotEqual,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::BranchGreaterThan(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::GreaterThan,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::GreaterThan,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::BranchGreaterThanOrEqual(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::GreaterThanOrEqual,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::GreaterThanOrEqual,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::BranchLessThan(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::LessThan,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::LessThan,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::BranchLessThanOrEqual(target) => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::LessThanOrEqual,
                            Type::Int32,
                            self.branch_labels[target],
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            JumpCondition::LessThanOrEqual,
                            Type::Float32,
                            self.branch_labels[target],
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
        }
    }

    pub fn done(self) -> Vec<InstructionIR> {
        self.instructions
    }
}
