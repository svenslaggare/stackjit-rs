use crate::model::function::Function;
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::compiler::FunctionCompilationData;
use crate::compiler::stack_layout;
use crate::compiler::calling_conventions::{CallingConventions};
use crate::compiler::binder::Binder;
use crate::ir::{InstructionIR, VirtualRegister};

pub struct InstructionIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_data: &'a mut FunctionCompilationData,
    instructions: Vec<InstructionIR>
}

impl<'a> InstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function, compilation_data: &'a mut FunctionCompilationData) -> InstructionIRCompiler<'a> {
        InstructionIRCompiler {
            binder,
            function,
            compilation_data,
            instructions: Vec::new()
        }
    }

    pub fn compile(&mut self, instructions: &Vec<Instruction>) {
        self.compile_initialize_function();

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
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
            self.instructions.push(InstructionIR::LoadZeroToRegister(VirtualRegister::Int(0)));
            for local_index in 0..(num_locals as u32) {
                let local_offset = stack_layout::local_stack_offset(self.function, local_index);
                self.instructions.push(InstructionIR::StoreMemory(local_offset, VirtualRegister::Int(0)));
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &Instruction) {
        self.instructions.push(InstructionIR::Marker(instruction_index));

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
                self.instructions.push(InstructionIR::LoadMemory(VirtualRegister::Int(0), local_offset));
                self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Int(0)));
            }
            Instruction::StoreLocal(index) => {
                let local_offset = stack_layout::local_stack_offset(self.function, *index);
                self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemory(local_offset, VirtualRegister::Int(0)));
            }
            Instruction::Add => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Int(0)));
                        self.instructions.push(InstructionIR::AddInt32(VirtualRegister::Int(0), VirtualRegister::Int(1)));
                        self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Int(0)));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Float(0)));
                        self.instructions.push(InstructionIR::AddFloat32(VirtualRegister::Float(0), VirtualRegister::Float(1)));
                        self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Float(0)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Sub => {
                match &self.function.instruction_operand_types(instruction_index)[0] {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Int(1)));
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Int(0)));
                        self.instructions.push(InstructionIR::SubInt32(VirtualRegister::Int(0), VirtualRegister::Int(1)));
                        self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Int(0)));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Float(1)));
                        self.instructions.push(InstructionIR::PopOperand(VirtualRegister::Float(0)));
                        self.instructions.push(InstructionIR::SubFloat32(VirtualRegister::Float(0), VirtualRegister::Float(1)));
                        self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Float(0)));
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
                self.instructions.push(InstructionIR::LoadMemory(VirtualRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::PushOperand(VirtualRegister::Int(0)));
            }
            Instruction::Return => {
                CallingConventions::new().make_return_value(self.function, &mut self.instructions);
                self.instructions.push(InstructionIR::Return);
            }
        }
    }

    pub fn done(self) -> Vec<InstructionIR> {
        self.instructions
    }
}
