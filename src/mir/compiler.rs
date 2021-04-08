use std::collections::HashMap;

use crate::optimization::null_check_elision::InstructionsRegisterNullStatus;
use crate::analysis::VirtualRegister;
use crate::compiler::ir::Condition;
use crate::model::binder::Binder;
use crate::mir::{InstructionMIR, RegisterMIR};
use crate::mir::branches::BranchManager;
use crate::mir::InstructionMIRData;
use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{TypeId, TypeStorage};
use crate::model::verifier::Verifier;

pub struct MIRCompilationResult {
    pub instructions: Vec<InstructionMIR>,
    pub num_virtual_registers: usize,
    pub local_virtual_registers: Vec<RegisterMIR>,
    pub need_zero_initialize_registers: Vec<RegisterMIR>,
    pub instructions_operand_stack: Vec<Vec<RegisterMIR>>
}

pub struct InstructionMIRCompiler<'a> {
    binder: &'a Binder,
    function: &'a Function,
    instructions: Vec<InstructionMIR>,
    branch_manager: BranchManager,
    local_virtual_registers: Vec<RegisterMIR>,
    next_operand_virtual_register: u32,
    max_num_virtual_register: usize,
    instructions_operands: Vec<Vec<RegisterMIR>>,
    macros: HashMap<FunctionSignature, Box<dyn Fn(&mut InstructionMIRCompiler, usize, &Instruction) + 'a>>
}

impl<'a> InstructionMIRCompiler<'a> {
    pub fn new(binder: &'a Binder, function: &'a Function) -> InstructionMIRCompiler<'a> {
        let mut compiler = InstructionMIRCompiler {
            binder,
            function,
            branch_manager: BranchManager::new(),
            instructions: Vec::new(),
            local_virtual_registers: Vec::new(),
            next_operand_virtual_register: 0,
            max_num_virtual_register: 0,
            instructions_operands: Vec::new(),
            macros: HashMap::new()
        };

        compiler.make_macro(
            FunctionSignature { name: "std.gc.collect".to_string(), parameters: vec![] },
            |compiler: &mut InstructionMIRCompiler, instruction_index: usize, _instruction: &Instruction| {
                compiler.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::GarbageCollect
                ));
            }
        );

        compiler.make_macro(
            FunctionSignature { name: "std.gc.print_stack_frame".to_string(), parameters: vec![] },
            |compiler: &mut InstructionMIRCompiler, instruction_index: usize, _instruction: &Instruction| {
                compiler.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::PrintStackFrame
                ));
            }
        );

        compiler
    }

    fn make_macro<F: Fn(&mut InstructionMIRCompiler, usize, &Instruction) + 'a>(&mut self, signature: FunctionSignature, f: F) {
        self.macros.insert(signature, Box::new(f));
    }

    pub fn compile(&mut self, instructions: &Vec<Instruction>) {
        self.branch_manager.define_branch_labels(instructions);

        for local_type in self.function.locals() {
            self.local_virtual_registers.push(
                RegisterMIR::new(self.next_operand_virtual_register, local_type.clone())
            );

            self.next_operand_virtual_register += 1;
        }

        self.max_num_virtual_register = self.local_virtual_registers.len();

        for (instruction_index, instruction) in instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &Instruction) {
        let operand_types = self.function.instruction_operand_types(instruction_index);

        if let Some(branch_label) = self.branch_manager.is_branch(instruction_index) {
            self.instructions_operands.push(Vec::new());
            self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::BranchLabel(branch_label)));
        }

        self.instructions_operands.push(
            operand_types
                .iter()
                .enumerate()
                .map(|(i, op_type)| RegisterMIR::new((self.local_virtual_registers.len() + i) as u32, op_type.clone()))
                .collect()
        );

        match instruction {
            Instruction::LoadInt32(value) => {
                let assign_reg = self.assign_stack_register(TypeId::Int32);
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadInt32(assign_reg, *value)));
            }
            Instruction::LoadFloat32(value) => {
                let assign_reg = self.assign_stack_register(TypeId::Float32);
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadFloat32(assign_reg, *value)));
            }
            Instruction::LoadTrue => {
                let assign_reg = self.assign_stack_register(TypeId::Bool);
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadBool(assign_reg, true)));
            }
            Instruction::LoadFalse => {
                let assign_reg = self.assign_stack_register(TypeId::Bool);
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadBool(assign_reg, false)));
            }
            Instruction::LoadLocal(index) => {
                let local_reg = self.local_virtual_registers[*index as usize].clone();
                let assign_reg = self.assign_stack_register(local_reg.value_type.clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::Move(assign_reg, local_reg)));
            }
            Instruction::StoreLocal(index) => {
                let local_reg = self.local_virtual_registers[*index as usize].clone();
                let value_reg = self.use_stack_register(local_reg.value_type.clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::Move(local_reg, value_reg)));
            }
            Instruction::Add => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Int32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::AddInt32(assign_reg, op1_reg, op2_reg)));
                    }
                    TypeId::Float32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::AddFloat32(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Sub => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Int32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::SubInt32(assign_reg, op1_reg, op2_reg)));
                    }
                    TypeId::Float32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::SubFloat32(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Multiply => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Int32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::MultiplyInt32(assign_reg, op1_reg, op2_reg)));
                    }
                    TypeId::Float32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::MultiplyFloat32(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Divide => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Int32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::DivideInt32(assign_reg, op1_reg, op2_reg)));
                    }
                    TypeId::Float32 => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::DivideFloat32(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::And => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Bool => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::AndBool(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Or => {
                let value_type = &operand_types[0];
                let op2_reg = self.use_stack_register(value_type.clone());
                let op1_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Bool => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::OrBool(assign_reg, op1_reg, op2_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Not => {
                let value_type = &operand_types[0];
                let op_reg = self.use_stack_register(value_type.clone());
                let assign_reg = self.assign_stack_register(value_type.clone());

                match value_type {
                    TypeId::Bool => {
                        self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::NotBool(assign_reg, op_reg)));
                    }
                    _ => { panic!("unexpected."); }
                }
            }
            Instruction::Return => {
                let return_value = if self.function.declaration().return_type() != &TypeId::Void {
                    Some(self.use_stack_register(self.function.declaration().return_type().clone()))
                } else {
                    None
                };

                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::Return(return_value)));
            }
            Instruction::Call(signature) => {
                match self.macros.remove(signature) {
                    Some(macro_fn) => {
                        macro_fn(self, instruction_index, instruction);
                        self.macros.insert(signature.clone(), macro_fn);
                    }
                    None => {
                        let func_to_call = self.binder.get(signature).unwrap();

                        let mut arguments_regs = func_to_call.parameters()
                            .iter().rev()
                            .map(|parameter| self.use_stack_register(parameter.clone()))
                            .collect::<Vec<_>>();
                        arguments_regs.reverse();

                        let return_value_reg = if func_to_call.return_type() != &TypeId::Void {
                            Some(self.assign_stack_register(func_to_call.return_type().clone()))
                        } else {
                            None
                        };

                        self.instructions.push(InstructionMIR::new(
                            instruction_index,
                            InstructionMIRData::Call(func_to_call.signature(), return_value_reg, arguments_regs)
                        ));
                    }
                }
            }
            Instruction::LoadArgument(argument_index) => {
                let assign_reg = self.assign_stack_register(self.function.declaration().parameters()[*argument_index as usize].clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadArgument(*argument_index, assign_reg)));
            }
            Instruction::LoadNull(null_type) => {
                let assign_reg = self.assign_stack_register(null_type.clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadNull(assign_reg)));
            }
            Instruction::NewArray(element) => {
                let size_reg = self.use_stack_register(TypeId::Int32);
                let assign_reg = self.assign_stack_register(TypeId::Array(Box::new(element.clone())));
                self.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::NewArray(element.clone(), assign_reg, size_reg)
                ));
            }
            Instruction::LoadElement(element) => {
                let index_reg = self.use_stack_register(TypeId::Int32);
                let array_ref_reg = self.use_stack_register(TypeId::Array(Box::new(element.clone())));
                let assign_reg = self.assign_stack_register(element.clone());
                self.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::LoadElement(element.clone(), assign_reg, array_ref_reg, index_reg)
                ));
            }
            Instruction::StoreElement(element) => {
                let value_ref = self.use_stack_register(element.clone());
                let index_reg = self.use_stack_register(TypeId::Int32);
                let array_ref_reg = self.use_stack_register(TypeId::Array(Box::new(element.clone())));
                self.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::StoreElement(element.clone(), array_ref_reg, index_reg, value_ref)
                ));
            }
            Instruction::LoadArrayLength => {
                let array_ref_reg = self.use_stack_register(operand_types[0].clone());
                let assign_reg = self.assign_stack_register(TypeId::Int32);
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::LoadArrayLength(assign_reg, array_ref_reg)));
            }
            Instruction::NewObject(class_type) => {
                let class_type = TypeId::Class(class_type.clone());
                let assign_reg = self.assign_stack_register(class_type.clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::NewObject(class_type, assign_reg)));
            }
            Instruction::LoadField(class_type, field_name) => {
                let class_type = TypeId::Class(class_type.clone());
                let class_ref_reg = self.use_stack_register(class_type.clone());
                let assign_reg = self.assign_stack_register(class_type.clone());
                self.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::LoadField(class_type, field_name.clone(), assign_reg, class_ref_reg)
                ));
            }
            Instruction::StoreField(class_type, field_name) => {
                let class_type = TypeId::Class(class_type.clone());
                let value_reg = self.use_stack_register(class_type.clone());
                let class_ref_reg = self.use_stack_register(class_type.clone());
                self.instructions.push(InstructionMIR::new(
                    instruction_index,
                    InstructionMIRData::StoreField(class_type, field_name.clone(), class_ref_reg, value_reg)
                ));
            }
            Instruction::Branch(target) => {
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::Branch(self.branch_manager.get_label(*target).unwrap())));
            }
            Instruction::BranchEqual(target)
            | Instruction::BranchNotEqual(target)
            | Instruction::BranchGreaterThan(target)
            | Instruction::BranchGreaterThanOrEqual(target)
            | Instruction::BranchLessThan(target)
            | Instruction::BranchLessThanOrEqual(target) => {
                let condition = match instruction {
                    Instruction::BranchEqual(_) => Condition::Equal,
                    Instruction::BranchNotEqual(_) => Condition::NotEqual,
                    Instruction::BranchGreaterThan(_) => Condition::GreaterThan,
                    Instruction::BranchGreaterThanOrEqual(_) => Condition::GreaterThanOrEqual,
                    Instruction::BranchLessThan(_) => Condition::LessThan,
                    Instruction::BranchLessThanOrEqual(_) => Condition::LessThanOrEqual,
                    _ => { panic!("unexpected."); }
                };

                let compare_type = operand_types[0].clone();
                let label = self.branch_manager.get_label(*target).unwrap();
                let op2_reg = self.use_stack_register(compare_type.clone());
                let op1_reg = self.use_stack_register(compare_type.clone());
                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::BranchCondition(
                    condition,
                    compare_type,
                    label,
                    op1_reg,
                    op2_reg
                )));
            }
            Instruction::CompareEqual
            | Instruction::CompareNotEqual
            | Instruction::CompareGreaterThan
            | Instruction::CompareGreaterThanOrEqual
            | Instruction::CompareLessThan
            | Instruction::CompareLessThanOrEqual => {
                let condition = match instruction {
                    Instruction::CompareEqual => Condition::Equal,
                    Instruction::CompareNotEqual => Condition::NotEqual,
                    Instruction::CompareGreaterThan => Condition::GreaterThan,
                    Instruction::CompareGreaterThanOrEqual => Condition::GreaterThanOrEqual,
                    Instruction::CompareLessThan => Condition::LessThan,
                    Instruction::CompareLessThanOrEqual => Condition::LessThanOrEqual,
                    _ => { panic!("unexpected."); }
                };

                let compare_type = operand_types[0].clone();
                let op2_reg = self.use_stack_register(compare_type.clone());
                let op1_reg = self.use_stack_register(compare_type.clone());
                let assign_reg = self.assign_stack_register(TypeId::Bool);

                self.instructions.push(InstructionMIR::new(instruction_index, InstructionMIRData::Compare(
                    condition,
                    compare_type,
                    assign_reg,
                    op1_reg,
                    op2_reg
                )));
            }
        }
    }

    fn use_stack_register(&mut self, value_type: TypeId) -> RegisterMIR {
        if self.next_operand_virtual_register == 0 {
            panic!("Invalid stack virtual register.");
        }

        self.next_operand_virtual_register -= 1;
        let number = self.next_operand_virtual_register;
        RegisterMIR::new(number, value_type)
    }

    fn assign_stack_register(&mut self, value_type: TypeId) -> RegisterMIR {
        let number = self.next_operand_virtual_register;
        self.next_operand_virtual_register += 1;
        self.max_num_virtual_register = self.max_num_virtual_register.max(self.next_operand_virtual_register as usize);
        RegisterMIR::new(number, value_type)
    }

    pub fn done(self) -> MIRCompilationResult {
        MIRCompilationResult {
            instructions: self.instructions,
            num_virtual_registers: self.max_num_virtual_register,
            local_virtual_registers: self.local_virtual_registers.clone(),
            need_zero_initialize_registers: self.local_virtual_registers.clone(),
            instructions_operand_stack: self.instructions_operands
        }
    }
}

#[test]
fn test_simple1() {
    let mut function = Function::new(
        FunctionDeclaration::with_manager("test".to_owned(), vec![], TypeId::Int32),
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

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done().instructions);
}

#[test]
fn test_simple2() {
    let mut function = Function::new(
        FunctionDeclaration::with_manager("test".to_owned(), vec![], TypeId::Int32),
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

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done().instructions);
}

#[test]
fn test_simple3() {
    let mut function = Function::new(
        FunctionDeclaration::with_manager("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
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

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done().instructions);
}

#[test]
fn test_simple4() {
    let mut function = Function::new(
        FunctionDeclaration::with_manager("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done().instructions);
}

#[test]
fn test_simple5() {
    let mut function = Function::new(
        FunctionDeclaration::with_manager("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadFloat32(2000.0),
            Instruction::Call(FunctionSignature::new("add".to_owned(), vec![TypeId::Int32, TypeId::Float32])),
            Instruction::Return
        ]
    );

    let mut binder = Binder::new();
    binder.define(FunctionDeclaration::with_manager(
        "add".to_owned(),
        vec![TypeId::Int32, TypeId::Float32],
        TypeId::Int32
    ));

    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());

    println_vec(function.instructions(), &compiler.done().instructions);
}

fn println_vec(original: &Vec<Instruction>, irs: &Vec<InstructionMIR>) {
    for ir in irs {
        println!("{:?}", original[ir.index]);
        println!("\t{:?}", ir.data);
    }
}