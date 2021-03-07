use crate::ir::mid::{InstructionMIR, VirtualRegister};
use crate::ir::low::{InstructionIR, HardwareRegister, HardwareRegisterExplicit, Variable};
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::verifier::Verifier;
use crate::engine::binder::Binder;
use crate::compiler::{FunctionCompilationData, stack_layout};
use crate::ir::mid::compiler::InstructionMIRCompiler;
use crate::model::typesystem::Type;
use crate::compiler::calling_conventions::{register_call_arguments, float_register_call_arguments, CallingConventions};

pub struct InstructionMIRToIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_data: &'a mut FunctionCompilationData,
    instructions: Vec<InstructionIR>
}

impl<'a> InstructionMIRToIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function, compilation_data: &'a mut FunctionCompilationData) -> InstructionMIRToIRCompiler<'a> {
        InstructionMIRToIRCompiler {
            binder,
            function,
            compilation_data,
            instructions: Vec::new()
        }
    }

    pub fn compile(&mut self, instructions: &Vec<InstructionMIR>) {
        self.compile_initialize_function();

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_initialize_function(&mut self) {
        self.instructions.push(InstructionIR::InitializeFunction);

        let stack_size = stack_layout::aligned_stack_size(self.function);
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

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &InstructionMIR) {
        match instruction {
            InstructionMIR::Marker(index) => {
                self.instructions.push(InstructionIR::Marker(*index));
            }
            InstructionMIR::LoadInt32(destination, value) => {
                self.instructions.push(InstructionIR::MoveInt32ToMemory(self.get_stack_offset(destination), *value));
            }
            InstructionMIR::Move(destination, source) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(source)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIR::AddInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIR::SubInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIR::AddFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIR::SubFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIR::Return(source) => {
                if let Some(source) = source {
                    CallingConventions::new().make_return_value(
                        self.function,
                        &Variable::Memory(self.get_stack_offset(source)),
                        &mut self.instructions
                    );
                }

                self.instructions.push(InstructionIR::Return);
            }
            InstructionMIR::Call(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let arguments_source = arguments
                    .iter()
                    .map(|argument| Variable::Memory(self.get_stack_offset(argument)))
                    .collect::<Vec<_>>();

                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source));

                if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &Variable::Memory(self.get_stack_offset(return_value)),
                        func_to_call,
                        &mut self.instructions
                    );
                }
            }
            InstructionMIR::LoadArgument(argument_index, destination) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                let register_offset = self.get_stack_offset(destination);

                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::StoreMemory(register_offset, HardwareRegister::Int(0)));
            }
            InstructionMIR::LoadNull(destination) => {
                self.instructions.push(InstructionIR::MoveInt32ToMemory(self.get_stack_offset(destination), 0));
            }
            InstructionMIR::NewArray(element, destination, size) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(size)));
                self.instructions.push(InstructionIR::NewArray(element.clone(), HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIR::LoadElement(element, destination, array_ref, index) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(array_ref)));

                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::LoadElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIR::StoreElement(element, array_ref, index, value) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(2), self.get_stack_offset(value)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(array_ref)));

                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::StoreElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1), HardwareRegister::Int(2)));
            }
            InstructionMIR::LoadArrayLength(destination, array_ref) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(array_ref)));
                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::LoadArrayLength(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIR::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIR::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIR::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                match compare_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_stack_offset(operand2)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            *condition,
                            Type::Int32,
                            *label,
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_stack_offset(operand2)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            *condition,
                            Type::Float32,
                            *label,
                            HardwareRegister::Float(0),
                            HardwareRegister::Float(1)
                        ));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
        }
    }

    fn get_stack_offset(&self, register: &VirtualRegister) -> i32 {
        -stack_layout::STACK_ENTRY_SIZE
        * (stack_layout::STACK_OFFSET + self.function.definition().parameters().len() as u32 + register.number) as i32
    }

    pub fn done(self) -> Vec<InstructionIR> {
        self.instructions
    }
}

#[test]
fn test_simple1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::Add,
            Instruction::LoadInt32(3),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compilation_data = FunctionCompilationData::new();

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_compiler.compile(function.instructions());
    let instructions_mir = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionMIRToIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_to_ir_compiler.compile(&instructions_mir);
    let instructions_ir = mir_to_ir_compiler.done();

    println_vec(function.instructions(), &instructions_ir);
}

#[test]
fn test_simple2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::Add,
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compilation_data = FunctionCompilationData::new();

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_compiler.compile(function.instructions());
    let instructions_mir = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionMIRToIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_to_ir_compiler.compile(&instructions_mir);
    let instructions_ir = mir_to_ir_compiler.done();

    println_vec(function.instructions(), &instructions_ir);
}

#[test]
fn test_simple3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32, Type::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(2000),
            Instruction::StoreLocal(1),
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,
            Instruction::LoadInt32(3000),
            Instruction::Add,
            Instruction::Return
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compilation_data = FunctionCompilationData::new();

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_compiler.compile(function.instructions());
    let instructions_mir = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionMIRToIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_to_ir_compiler.compile(&instructions_mir);
    let instructions_ir = mir_to_ir_compiler.done();

    println_vec(function.instructions(), &instructions_ir);
}

#[test]
fn test_simple4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    );

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compilation_data = FunctionCompilationData::new();

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_compiler.compile(function.instructions());
    let instructions_mir = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionMIRToIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_to_ir_compiler.compile(&instructions_mir);
    let instructions_ir = mir_to_ir_compiler.done();

    println_vec(function.instructions(), &instructions_ir);
}

#[test]
fn test_simple5() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadFloat32(2000.0),
            Instruction::Call(FunctionSignature::new("add".to_owned(), vec![Type::Int32, Type::Float32])),
            Instruction::Return
        ]
    );

    let mut binder = Binder::new();
    binder.define(FunctionDefinition::new_managed(
        "add".to_owned(),
        vec![Type::Int32, Type::Float32],
        Type::Int32
    ));

    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compilation_data = FunctionCompilationData::new();

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_compiler.compile(function.instructions());
    let instructions_mir = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionMIRToIRCompiler::new(&binder, &function, &mut compilation_data);
    mir_to_ir_compiler.compile(&instructions_mir);
    let instructions_ir = mir_to_ir_compiler.done();

    println_vec(function.instructions(), &instructions_ir);
}


fn println_vec(original: &Vec<Instruction>, irs: &Vec<InstructionIR>) {
    for ir in irs {
        match ir {
            InstructionIR::Marker(index) => {
                println!("{:?}", original[*index]);
            }
            instruction => {
                println!("\t{:?}", instruction);
            }
        }
    }
}
