use std::collections::HashMap;

use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::compiler::FunctionCompilationData;
use crate::ir::mid::{VirtualRegister, InstructionMIR};
use crate::engine::binder::Binder;
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::model::verifier::Verifier;
use crate::ir::branches::BranchManager;
use crate::ir::low::JumpCondition;

pub struct InstructionMIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    compilation_data: &'a mut FunctionCompilationData,
    instructions: Vec<InstructionMIR>,
    branch_manager: BranchManager,
    local_virtual_registers: HashMap<u32, VirtualRegister>,
    next_stack_virtual_register: u32
}

impl<'a> InstructionMIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function, compilation_data: &'a mut FunctionCompilationData) -> InstructionMIRCompiler<'a> {
        InstructionMIRCompiler {
            binder,
            function,
            compilation_data,
            branch_manager: BranchManager::new(),
            instructions: Vec::new(),
            local_virtual_registers: HashMap::new(),
            next_stack_virtual_register: 0
        }
    }

    pub fn compile(&mut self, instructions: &Vec<Instruction>) {
        self.branch_manager.define_branch_labels(instructions);

        for (local_index, local_type) in self.function.locals().iter().enumerate() {
            self.local_virtual_registers.insert(local_index as u32, VirtualRegister::new(self.next_stack_virtual_register, local_type.clone()));
            self.next_stack_virtual_register += 1;
        }

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &Instruction) {
        let operand_types = self.function.instruction_operand_types(instruction_index);

        self.instructions.push(InstructionMIR::Marker(instruction_index));

        if let Some(branch_label) = self.branch_manager.is_branch(instruction_index) {
            self.instructions.push(InstructionMIR::BranchLabel(branch_label));
        }

        match instruction {
            Instruction::LoadInt32(value) => {
                let assign_reg = self.assign_stack_register(Type::Int32);
                self.instructions.push(InstructionMIR::LoadInt32(assign_reg, *value));
            }
            Instruction::LoadFloat32(value) => {
                let assign_reg = self.assign_stack_register(Type::Int32);
                let value: i32 = unsafe { std::mem::transmute(*value) };
                self.instructions.push(InstructionMIR::LoadInt32(assign_reg, value));
            }
            Instruction::LoadLocal(index) => {
                let local_reg = self.local_virtual_registers[index].clone();
                let assign_reg = self.assign_stack_register(local_reg.value_type.clone());
                self.instructions.push(InstructionMIR::Move(assign_reg, local_reg));
            }
            Instruction::StoreLocal(index) => {
                let local_reg = self.local_virtual_registers[index].clone();
                let value_reg = self.use_stack_register(local_reg.value_type.clone());
                self.instructions.push(InstructionMIR::Move(local_reg, value_reg));
            }
            Instruction::Add => {
                let value_type = &operand_types[0].value_type;
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionMIR::AddInt32(assign_reg, op1_reg, op2_reg));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionMIR::AddFloat32(assign_reg, op1_reg, op2_reg));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Sub => {
                let value_type = &operand_types[0].value_type;
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionMIR::SubInt32(assign_reg, op1_reg, op2_reg));
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionMIR::SubFloat32(assign_reg, op1_reg, op2_reg));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Return => {
                let return_value = if self.function.definition().return_type() != &Type::Void {
                    Some(self.use_stack_register(self.function.definition().return_type().clone()))
                } else {
                    None
                };

                self.instructions.push(InstructionMIR::Return(return_value));
            }
            Instruction::Call(signature) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let mut arguments_regs = func_to_call.parameters()
                    .iter().rev()
                    .map(|parameter| self.use_stack_register(parameter.clone()))
                    .collect::<Vec<_>>();
                arguments_regs.reverse();

                let return_value_reg = if func_to_call.return_type() != &Type::Void {
                    Some(self.assign_stack_register(func_to_call.return_type().clone()))
                } else {
                    None
                };

                self.instructions.push(InstructionMIR::Call(func_to_call.call_signature(), return_value_reg, arguments_regs));
            }
            Instruction::LoadArgument(argument_index) => {
                let assign_reg = self.assign_stack_register(self.function.definition().parameters()[*argument_index as usize].clone());
                self.instructions.push(InstructionMIR::LoadArgument(*argument_index, assign_reg));
            }
            Instruction::LoadNull => {
                let assign_reg = self.assign_stack_register(Type::Null);
                self.instructions.push(InstructionMIR::LoadNull(assign_reg));
            }
            Instruction::NewArray(element) => {
                let size_reg = self.use_stack_register(Type::Int32);
                let assign_reg = self.assign_stack_register(Type::Array(Box::new(element.clone())));
                self.instructions.push(InstructionMIR::NewArray(element.clone(), assign_reg, size_reg));
            }
            Instruction::LoadElement(element) => {
                let index_reg = self.use_stack_register(Type::Int32);
                let array_ref_reg = self.use_stack_register(Type::Array(Box::new(element.clone())));
                let assign_reg = self.assign_stack_register(element.clone());
                self.instructions.push(InstructionMIR::LoadElement(element.clone(), assign_reg, array_ref_reg, index_reg))
            }
            Instruction::StoreElement(element) => {
                let value_ref = self.use_stack_register(element.clone());
                let index_reg = self.use_stack_register(Type::Int32);
                let array_ref_reg = self.use_stack_register(Type::Array(Box::new(element.clone())));
                self.instructions.push(InstructionMIR::StoreElement(element.clone(), array_ref_reg, index_reg, value_ref));
            }
            Instruction::LoadArrayLength => {
                let array_ref_reg = self.use_stack_register(operand_types[0].value_type.clone());
                let assign_reg = self.assign_stack_register(Type::Int32);
                self.instructions.push(InstructionMIR::LoadArrayLength(assign_reg, array_ref_reg));
            }
            Instruction::Branch(target) => {
                self.instructions.push(InstructionMIR::Branch(self.branch_manager.get_label(*target).unwrap()));
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

                let compare_type = operand_types[0].value_type.clone();
                let label = self.branch_manager.get_label(*target).unwrap();
                let op2_reg = self.use_stack_register(compare_type.clone());
                let op1_reg = self.use_stack_register(compare_type.clone());
                self.instructions.push(InstructionMIR::BranchCondition(
                    condition,
                    compare_type,
                    label,
                    op1_reg,
                    op2_reg
                ));
            }
        }
    }

    fn use_stack_register(&mut self, value_type: Type) -> VirtualRegister {
        if self.next_stack_virtual_register == 0 {
            panic!("Invalid stack virtual register.");
        }

        self.next_stack_virtual_register -= 1;
        let number = self.next_stack_virtual_register;
        VirtualRegister::new(number, value_type)
    }

    fn assign_stack_register(&mut self, value_type: Type) -> VirtualRegister {
        let number = self.next_stack_virtual_register;
        self.next_stack_virtual_register += 1;
        VirtualRegister::new(number, value_type)
    }

    pub fn done(self) -> Vec<InstructionMIR> {
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

    let mut compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done());
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

    let mut compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done());
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

    let mut compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done());
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

    let mut compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done());
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

    let mut compiler = InstructionMIRCompiler::new(&binder, &function, &mut compilation_data);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done());
}

fn println_vec(original: &Vec<Instruction>, irs: &Vec<InstructionMIR>) {
    for ir in irs {
        match ir {
            InstructionMIR::Marker(index) => {
                println!("{:?}", original[*index]);
            }
            instruction => {
                println!("\t{:?}", instruction);
            }
        }
    }
}