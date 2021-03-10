use crate::ir::low::{InstructionIR, HardwareRegister, HardwareRegisterExplicit, Variable};
use crate::ir::mid::{InstructionMIRData, VirtualRegister, InstructionMIR};
use crate::ir::mid::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::verifier::Verifier;
use crate::model::typesystem::Type;
use crate::compiler::{stack_layout};
use crate::compiler::calling_conventions::{register_call_arguments, CallingConventions};
use crate::engine::binder::Binder;

pub struct InstructionIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    instructions: Vec<InstructionIR>
}

impl<'a> InstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function) -> InstructionIRCompiler<'a> {
        InstructionIRCompiler {
            binder,
            function,
            instructions: Vec::new()
        }
    }

    pub fn compile(&mut self, mir_result: &MIRCompilationResult) {
        self.compile_initialize_function(mir_result);

        for (instruction_index, instruction) in mir_result.instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_initialize_function(&mut self, mir_result: &MIRCompilationResult) {
        self.instructions.push(InstructionIR::InitializeFunction);

        let stack_size = stack_layout::align_size(stack_layout::stack_size_mir(self.function, mir_result));
        if stack_size > 0 {
            self.instructions.push(InstructionIR::SubFromStackPointer(stack_size));
        }

        CallingConventions::new().move_arguments_to_stack(self.function, &mut self.instructions);

        if !mir_result.need_zero_initialize_registers.is_empty() {
            self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::Int(0)));
            for register in &mir_result.need_zero_initialize_registers {
                self.instructions.push(InstructionIR::StoreMemory(
                    self.get_register_stack_offset(register),
                    HardwareRegister::Int(0)
                ));
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &InstructionMIR) {
        self.instructions.push(InstructionIR::Marker(instruction.index));

        match &instruction.data {
            InstructionMIRData::LoadInt32(destination, value) => {
                self.instructions.push(InstructionIR::MoveInt32ToMemory(self.get_register_stack_offset(destination), *value));
            }
            InstructionMIRData::LoadFloat32(destination, value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };
                self.instructions.push(InstructionIR::MoveInt32ToMemory(self.get_register_stack_offset(destination), value));
            }
            InstructionMIRData::Move(destination, source) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(source)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::AddInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::SubInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::AddFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_register_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIRData::SubFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreMemory(self.get_register_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIRData::Return(source) => {
                if let Some(source) = source {
                    CallingConventions::new().make_return_value(
                        self.function,
                        &Variable::Memory(self.get_register_stack_offset(source)),
                        &mut self.instructions
                    );
                }

                self.instructions.push(InstructionIR::Return);
            }
            InstructionMIRData::Call(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let arguments_source = arguments
                    .iter()
                    .map(|argument| Variable::Memory(self.get_register_stack_offset(argument)))
                    .collect::<Vec<_>>();

                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source));

                if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &Variable::Memory(self.get_register_stack_offset(return_value)),
                        func_to_call,
                        &mut self.instructions
                    );
                }
            }
            InstructionMIRData::LoadArgument(argument_index, destination) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                let register_offset = self.get_register_stack_offset(destination);

                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::StoreMemory(register_offset, HardwareRegister::Int(0)));
            }
            InstructionMIRData::LoadNull(destination) => {
                self.instructions.push(InstructionIR::MoveInt32ToMemory(self.get_register_stack_offset(destination), 0));
            }
            InstructionMIRData::NewArray(element, destination, size) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(size)));
                self.instructions.push(InstructionIR::NewArray(element.clone(), HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_register_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIRData::LoadElement(element, destination, array_ref, index) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_register_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));

                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::LoadElement(element.clone(), HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_register_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIRData::StoreElement(element, array_ref, index, value) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(2), self.get_register_stack_offset(value)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_register_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));

                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::StoreElement(
                    element.clone(),
                    HardwareRegister::Int(0),
                    HardwareRegister::Int(1),
                    HardwareRegister::Int(2)
                ));
            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {
                self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));
                self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::LoadArrayLength(HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreMemoryExplicit(
                    self.get_register_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIRData::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIRData::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIRData::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                match compare_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                        self.instructions.push(InstructionIR::BranchCondition(
                            *condition,
                            Type::Int32,
                            *label,
                            HardwareRegister::Int(0),
                            HardwareRegister::Int(1)
                        ));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
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

    fn get_register_stack_offset(&self, register: &VirtualRegister) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function, register.number)
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

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
    mir_compiler.compile(function.instructions());
    let mir_result = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionIRCompiler::new(&binder, &function);
    mir_to_ir_compiler.compile(&mir_result);
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

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
    mir_compiler.compile(function.instructions());
    let mir_result = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionIRCompiler::new(&binder, &function);
    mir_to_ir_compiler.compile(&mir_result);
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

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
    mir_compiler.compile(function.instructions());
    let mir_result = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionIRCompiler::new(&binder, &function);
    mir_to_ir_compiler.compile(&mir_result);
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

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
    mir_compiler.compile(function.instructions());
    let mir_result = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionIRCompiler::new(&binder, &function);
    mir_to_ir_compiler.compile(&mir_result);
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

    let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
    mir_compiler.compile(function.instructions());
    let mir_result = mir_compiler.done();

    let mut mir_to_ir_compiler = InstructionIRCompiler::new(&binder, &function);
    mir_to_ir_compiler.compile(&mir_result);
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
