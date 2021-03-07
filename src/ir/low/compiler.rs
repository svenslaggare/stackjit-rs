use crate::model::function::Function;
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::ir::low::{HardwareRegister, InstructionIR, JumpCondition, Variable, HardwareRegisterExplicit};
use crate::ir::branches::BranchManager;
use crate::compiler::calling_conventions::{CallingConventions, register_call_arguments};
use crate::compiler::FunctionCompilationData;
use crate::compiler::stack_layout;
use crate::engine::binder::Binder;

pub struct InstructionIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_data: &'a mut FunctionCompilationData,
    instructions: Vec<InstructionIR>,
    branch_manager: BranchManager
}

impl<'a> InstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function, compilation_data: &'a mut FunctionCompilationData) -> InstructionIRCompiler<'a> {
        InstructionIRCompiler {
            binder,
            function,
            compilation_data,
            instructions: Vec::new(),
            branch_manager: BranchManager::new()
        }
    }

    pub fn compile(&mut self, instructions: &Vec<Instruction>) {
        self.compile_initialize_function();
        self.branch_manager.define_branch_labels(instructions);

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_initialize_function(&mut self) {
        self.instructions.push(InstructionIR::InitializeFunction);

        let stack_size = stack_layout::align_size(stack_layout::stack_size(self.function));
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

        if let Some(branch_label) = self.branch_manager.is_branch(instruction_index) {
            self.instructions.push(InstructionIR::BranchLabel(branch_label));
        }

        let operand_types = self.function.instruction_operand_types(instruction_index);

        match instruction {
            Instruction::LoadInt32(value) => {
                self.instructions.push(InstructionIR::LoadInt32(*value));
            }
            Instruction::LoadFloat32(value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };
                self.instructions.push(InstructionIR::LoadInt32(value));
            }
            Instruction::LoadNull => {
                self.instructions.push(InstructionIR::LoadInt32(0));
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
                match &operand_types[0].value_type {
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
                match &operand_types[0].value_type {
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
                let func_to_call = self.binder.get(signature).unwrap();
                let arguments = func_to_call.parameters().iter().map(|_| Variable::OperandStack).collect();
                self.instructions.push(InstructionIR::Call(signature.clone(), arguments));
                CallingConventions::new().handle_return_value(
                    self.function,
                    &Variable::OperandStack,
                    func_to_call,
                    &mut self.instructions
                );
            }
            Instruction::LoadArgument(argument_index) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::PushOperand(HardwareRegister::Int(0)));
            }
            Instruction::Return => {
                CallingConventions::new().make_return_value(
                    self.function,
                    &Variable::OperandStack,
                    &mut self.instructions
                );
                self.instructions.push(InstructionIR::Return);
            }
            Instruction::NewArray(element) => {
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::NewArray(element.clone(), HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::PushOperandExplicit(HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)));
            }
            Instruction::LoadElement(element) => {
                let is_non_null = &operand_types[0].non_null;

                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1))); // The index of the element
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0))); // The array reference

                if !is_non_null {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::LoadElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::PushOperandExplicit(HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)));
            }
            Instruction::StoreElement(element) => {
                let is_non_null = &operand_types[0].non_null;

                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(2))); // The value to store
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1))); // The index of the element
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0))); // The array reference

                if !is_non_null {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::StoreElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1), HardwareRegister::Int(2)));
            }
            Instruction::LoadArrayLength => {
                let is_non_null = &operand_types[0].non_null;
                self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0))); // The array reference

                if !is_non_null {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                self.instructions.push(InstructionIR::LoadArrayLength(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::PushOperandExplicit(HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)));
            }
            Instruction::Branch(target) => {
                self.instructions.push(InstructionIR::Branch(self.branch_manager.get_label(*target).unwrap()));
            }
            Instruction::BranchEqual(target)
            | Instruction::BranchNotEqual(target)
            | Instruction::BranchGreaterThan(target)
            | Instruction::BranchGreaterThanOrEqual(target)
            | Instruction::BranchLessThan(target)
            | Instruction::BranchLessThanOrEqual(target) => {
                let condition = match instruction {
                    Instruction::BranchEqual(_) => JumpCondition::Equal,
                    Instruction::BranchNotEqual(_) => JumpCondition::NotEqual,
                    Instruction::BranchGreaterThan(_) => JumpCondition::GreaterThan,
                    Instruction::BranchGreaterThanOrEqual(_) => JumpCondition::GreaterThanOrEqual,
                    Instruction::BranchLessThan(_) => JumpCondition::LessThan,
                    Instruction::BranchLessThanOrEqual(_) => JumpCondition::LessThanOrEqual,
                    _ => { panic!("unexpected."); }
                };

                let branch_label = self.branch_manager.get_label(*target).unwrap();
                match &operand_types[0].value_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Int(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            condition,
                            Type::Int32,
                            branch_label,
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(HardwareRegister::Float(0)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            condition,
                            Type::Float32,
                            branch_label,
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
