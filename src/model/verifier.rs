use crate::model::function::{Function, FunctionSignature, FunctionDefinition};
use crate::model::typesystem::Type;
use crate::model::instruction::Instruction;
use crate::engine::binder::Binder;

#[derive(Debug, PartialEq, Eq)]
pub struct VerifyError {
    pub index: Option<usize>,
    pub message: VerifyErrorMessage
}

impl VerifyError {
    pub fn new(message: VerifyErrorMessage) -> VerifyError {
        VerifyError {
            index: None,
            message
        }
    }

    pub fn with_index(index: usize, message: VerifyErrorMessage) -> VerifyError {
        VerifyError {
            index: Some(index),
            message
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyErrorMessage {
    EmptyOperandStack,
    NonEmptyOperandStackOnReturn,
    LocalIndexOutOfRange,
    ArgumentIndexOutOfRange,
    WrongType(Type, Type),
    WrongArithmeticOperands,
    FunctionNotDefined(FunctionSignature),
    ExpectedNumberOfOperands(usize),
    ParameterCannotBeVoid,
    LocalCannotBeVoid,
    InvalidBranchTarget,
    BranchDifferentNumberOfOperands(usize, usize),
    ExpectedComparableType,
    ExpectedArrayReference
}

pub type VerifyResult<T> = Result<T, VerifyError>;

pub struct Verifier<'a> {
    function: &'a mut Function,
    binder: &'a Binder,
    operand_stack: Vec<Type>,
    branches: Vec<(usize, usize, Vec<Type>)>
}

impl<'a> Verifier<'a> {
    pub fn new(binder: &'a Binder, function: &'a mut Function) -> Verifier<'a> {
        Verifier {
            function,
            binder,
            operand_stack: Vec::new(),
            branches: Vec::new()
        }
    }

    pub fn verify(&mut self) -> VerifyResult<()> {
        for parameter in self.function.definition().parameters() {
            if parameter == &Type::Void {
                return Err(VerifyError::new(VerifyErrorMessage::ParameterCannotBeVoid));
            }
        }

        self.verify_instructions()?;
        self.verify_locals()?;
        self.verify_branches()?;

        if !self.operand_stack.is_empty() {
            return Err(VerifyError::new(VerifyErrorMessage::NonEmptyOperandStackOnReturn));
        }

        Ok(())
    }

    fn verify_instructions(&mut self) -> VerifyResult<()> {
        let mut max_stack_size = 0;

        let instructions = (*self.function.instructions()).clone();
        for (instruction_index, instruction) in instructions.iter().enumerate() {
            *self.function.instruction_operand_types_mut(instruction_index) = self.operand_stack.clone();
            max_stack_size = max_stack_size.max(self.operand_stack.len());

            match instruction {
                Instruction::LoadInt32(_) => {
                    self.push_operand_stack(Type::Int32);
                }
                Instruction::LoadFloat32(_) => {
                    self.push_operand_stack(Type::Float32);
                }
                Instruction::LoadNull(null_type) => {
                    self.push_operand_stack(null_type.clone());
                }
                Instruction::LoadLocal(index) => {
                    let local_type = self.function.locals().get(*index as usize)
                        .ok_or(VerifyError::with_index(instruction_index, VerifyErrorMessage::LocalIndexOutOfRange))?
                        .clone();

                    self.push_operand_stack(local_type);
                }
                Instruction::StoreLocal(index) => {
                    let operand = self.pop_operand_stack(instruction_index)?;
                    let local_type = self.function.locals().get(*index as usize)
                        .ok_or(VerifyError::with_index(instruction_index, VerifyErrorMessage::LocalIndexOutOfRange))?
                        .clone();

                    self.same_type(instruction_index, &local_type, &operand)?;
                }
                Instruction::Add | Instruction::Sub => {
                    let op2 = self.pop_operand_stack(instruction_index)?;
                    let op1 = self.pop_operand_stack(instruction_index)?;
                    match (&op1, &op2) {
                        (Type::Int32, Type::Int32) => {
                            self.push_operand_stack(op1);
                        }
                        (Type::Float32, Type::Float32) => {
                            self.push_operand_stack(op1);
                        }
                        _ => {
                            return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::WrongArithmeticOperands));
                        }
                    }
                }
                Instruction::Call(signature) => {
                    let func_to_call = self.binder.get(signature)
                        .ok_or(VerifyError::with_index(instruction_index, VerifyErrorMessage::FunctionNotDefined(signature.clone())))?;

                    if self.operand_stack.len() < func_to_call.parameters().len() {
                        return Err(VerifyError::with_index(
                            instruction_index,
                            VerifyErrorMessage::ExpectedNumberOfOperands(func_to_call.parameters().len())
                        ));
                    }

                    for parameter in func_to_call.parameters().iter().rev() {
                        let operand = self.pop_operand_stack(instruction_index)?;
                        self.same_type(instruction_index, parameter, &operand)?;
                    }

                    if func_to_call.return_type() != &Type::Void {
                        self.push_operand_stack(func_to_call.return_type().clone());
                    }
                }
                Instruction::LoadArgument(index) => {
                    let argument_type = self.function.definition().parameters().get(*index as usize)
                        .ok_or(VerifyError::with_index(instruction_index, VerifyErrorMessage::ArgumentIndexOutOfRange))?
                        .clone();

                    self.push_operand_stack(argument_type);
                }
                Instruction::Return => {
                    if self.function.definition().return_type() != &Type::Void {
                        let operand = self.pop_operand_stack(instruction_index)?;
                        self.same_type(instruction_index, self.function.definition().return_type(), &operand)?;
                    }
                }
                Instruction::NewArray(element) => {
                    let length = self.pop_operand_stack(instruction_index)?;
                    self.same_type(instruction_index, &Type::Int32, &length)?;
                    self.push_operand_stack(Type::Array(Box::new(element.clone())));
                }
                Instruction::LoadElement(element) => {
                    let array_index = self.pop_operand_stack(instruction_index)?;
                    let array_reference = self.pop_operand_stack(instruction_index)?;
                    let array_reference_type = Type::Array(Box::new(element.clone()));

                    self.same_type(instruction_index, &Type::Int32, &array_index)?;
                    self.same_type(instruction_index, &array_reference_type, &array_reference)?;

                    self.push_operand_stack(element.clone());
                }
                Instruction::StoreElement(element) => {
                    let array_value = self.pop_operand_stack(instruction_index)?;
                    let array_index = self.pop_operand_stack(instruction_index)?;
                    let array_reference = self.pop_operand_stack(instruction_index)?;
                    let array_reference_type = Type::Array(Box::new(element.clone()));

                    self.same_type(instruction_index, &Type::Int32, &array_index)?;
                    self.same_type(instruction_index, &array_reference_type, &array_reference)?;
                    self.same_type(instruction_index, &array_value, &element)?;
                }
                Instruction::LoadArrayLength => {
                    let array_reference = self.pop_operand_stack(instruction_index)?;

                    if !array_reference.is_array() {
                        return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::ExpectedArrayReference));
                    }

                    self.push_operand_stack(Type::Int32);
                }
                Instruction::Branch(target) => {
                    if *target >= self.function.instructions().len() as u32 {
                        return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::InvalidBranchTarget));
                    }

                    self.branches.push((instruction_index, *target as usize, self.clone_operand_stack()));
                }
                Instruction::BranchEqual(target) | Instruction::BranchNotEqual(target) => {
                    let op2 = self.pop_operand_stack(instruction_index)?;
                    let op1 = self.pop_operand_stack(instruction_index)?;

                    self.same_type(instruction_index, &op1, &op2)?;
                    if *target >= self.function.instructions().len() as u32 {
                        return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::InvalidBranchTarget));
                    }

                    self.branches.push((instruction_index, *target as usize, self.clone_operand_stack()));
                }
                Instruction::BranchGreaterThan(target)
                | Instruction::BranchGreaterThanOrEqual(target)
                | Instruction::BranchLessThan(target)
                | Instruction::BranchLessThanOrEqual(target) => {
                    let op2 = self.pop_operand_stack(instruction_index)?;
                    let op1 = self.pop_operand_stack(instruction_index)?;

                    self.same_type(instruction_index, &op1, &op2)?;
                    if *target >= self.function.instructions().len() as u32 {
                        return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::InvalidBranchTarget));
                    }

                    match op1 {
                        Type::Int32 | Type::Float32 => {}
                        _ => {
                            return Err(VerifyError::with_index(instruction_index, VerifyErrorMessage::ExpectedComparableType));
                        }
                    }

                    self.branches.push((instruction_index, *target as usize, self.clone_operand_stack()));
                }
            }
        }

        self.function.set_operand_stack_size(max_stack_size);

        Ok(())
    }

    fn verify_locals(&mut self) -> VerifyResult<()> {
        for local in self.function.locals() {
            if local == &Type::Void {
                return Err(VerifyError::new(VerifyErrorMessage::LocalCannotBeVoid));
            }
        }

        Ok(())
    }

    fn verify_branches(&mut self) -> VerifyResult<()> {
        for (branch_source, branch_target, branch_source_operands) in &self.branches {
            let branch_target_operands = self.function.instruction_operand_types(*branch_target);

            if branch_source_operands.len() == branch_target_operands.len() {
                for index in 0..branch_source_operands.len() {
                    let source_type = &branch_source_operands[index];
                    let target_type = &branch_target_operands[index];
                    self.same_type(*branch_source, source_type, target_type)?;
                }
            } else {
                return Err(VerifyError::with_index(
                    *branch_source,
                    VerifyErrorMessage::BranchDifferentNumberOfOperands(branch_source_operands.len(), branch_target_operands.len())
                ));
            }
        }

        Ok(())
    }

    fn push_operand_stack(&mut self, value_type: Type) {
        self.operand_stack.push(value_type);
    }

    fn pop_operand_stack(&mut self, instruction_index: usize) -> VerifyResult<Type> {
        self.operand_stack.pop()
            .ok_or(VerifyError::with_index(instruction_index, VerifyErrorMessage::EmptyOperandStack))
    }

    fn clone_operand_stack(&self) -> Vec<Type> {
        self.operand_stack.iter().map(|o| o.clone()).collect()
    }

    fn same_type(&self, instruction_index: usize, expected: &Type, actual: &Type) -> VerifyResult<()> {
        if !expected.is_same_type(actual) {
            Err(VerifyError::with_index(
                instruction_index,
                VerifyErrorMessage::WrongType(expected.clone(), actual.clone())
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn test_simple1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_simple2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
    assert_eq!(&vec![Type::Int32, Type::Int32], function.instruction_operand_types(2));
}

#[test]
fn test_simple3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Float32),
        Vec::new(),
        vec![
            Instruction::LoadFloat32(47.11),
            Instruction::LoadFloat32(13.37),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
    assert_eq!(&vec![Type::Float32, Type::Float32], function.instruction_operand_types(2));
}

#[test]
fn test_return1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Err(VerifyError::new(VerifyErrorMessage::NonEmptyOperandStackOnReturn)), verifier.verify());
}

#[test]
fn test_return2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Err(VerifyError::new(VerifyErrorMessage::NonEmptyOperandStackOnReturn)), verifier.verify());
}


#[test]
fn test_return3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Float32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(3, VerifyErrorMessage::WrongType(Type::Float32, Type::Int32))),
        verifier.verify()
    );
}


#[test]
fn test_local1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_local2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(1, VerifyErrorMessage::LocalIndexOutOfRange)),
        verifier.verify()
    );
}

#[test]
fn test_call1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![Type::Int32], Type::Int32),
        vec![],
        vec![
            Instruction::LoadArgument(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
    assert_eq!(&vec![Type::Int32], function.instruction_operand_types(1));
}

#[test]
fn test_call2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![Type::Int32], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadFloat32(3.0),
            Instruction::LoadInt32(4),
            Instruction::LoadFloat32(5.0),
            Instruction::Call(FunctionSignature::new("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32])),
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    binder.define(FunctionDefinition::new_managed("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32], Type::Int32));

    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_call3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![Type::Int32], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadFloat32(5.0),
            Instruction::Call(FunctionSignature::new("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32])),
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    binder.define(FunctionDefinition::new_managed("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32], Type::Int32));

    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(3, VerifyErrorMessage::ExpectedNumberOfOperands(5))),
        verifier.verify()
    );
}

#[test]
fn test_call4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![Type::Int32], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::LoadInt32(4),
            Instruction::LoadFloat32(5.0),
            Instruction::Call(FunctionSignature::new("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32])),
            Instruction::Return,
        ]
    );

    let mut binder = Binder::new();
    binder.define(FunctionDefinition::new_managed("test_call".to_owned(), vec![Type::Int32, Type::Int32, Type::Float32, Type::Int32, Type::Float32], Type::Int32));

    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(5, VerifyErrorMessage::WrongType(Type::Float32, Type::Int32))),
        verifier.verify()
    );
}

#[test]
fn test_array1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_array2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Float32),
            Instruction::LoadInt32(1000),
            Instruction::LoadFloat32(47.11),
            Instruction::StoreElement(Type::Float32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_array3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(1000),
            Instruction::LoadFloat32(47.11),
            Instruction::StoreElement(Type::Float32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(4, VerifyErrorMessage::WrongType(Type::Array(Box::new(Type::Float32)), Type::Array(Box::new(Type::Int32))))),
        verifier.verify()
    );
}

#[test]
fn test_array4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(5411),
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(3, VerifyErrorMessage::WrongType(Type::Array(Box::new(Type::Int32)), Type::Int32))),
        verifier.verify()
    );
}

#[test]
fn test_array5() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Float32),
            Instruction::LoadArrayLength,
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::BranchNotEqual(5),
            Instruction::LoadInt32(3),
            Instruction::StoreLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_branches2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::BranchNotEqual(6),
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_branches3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        Vec::new(),
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::BranchNotEqual(6),
            Instruction::LoadInt32(4),
            Instruction::LoadInt32(5),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(3, VerifyErrorMessage::BranchDifferentNumberOfOperands(1, 3))),
        verifier.verify()
    );
}

#[test]
fn test_branches4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Void),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::BranchNotEqual(5),
            Instruction::LoadInt32(4),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(3, VerifyErrorMessage::BranchDifferentNumberOfOperands(1, 2))),
        verifier.verify()
    );
}

#[test]
fn test_branches5() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::BranchLessThanOrEqual(6),
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());
}

#[test]
fn test_branches6() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(0),
            Instruction::BranchLessThanOrEqual(6),
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(
        Err(VerifyError::with_index(2, VerifyErrorMessage::ExpectedComparableType)),
        verifier.verify()
    );
}

#[test]
fn test_null1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());

    assert!(function.instruction_operand_types(1)[0].is_reference());
}

#[test]
fn test_null2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());

    assert!(function.instruction_operand_types(2)[0].is_reference());
}

#[test]
fn test_null3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(4711),
            Instruction::StoreElement(Type::Int32),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let mut verifier = Verifier::new(&binder, &mut function);
    assert_eq!(Ok(()), verifier.verify());

    assert!(function.instruction_operand_types(3)[0].is_reference());
}