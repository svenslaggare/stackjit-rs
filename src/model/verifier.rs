use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::typesystem::Type;
use crate::model::instruction::Instruction;
use crate::compiler::binder::Binder;

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    EmptyOperandStack,
    NonEmptyOperandStackOnReturn,
    LocalIndexOutOfRange,
    ArgumentIndexOutOfRange,
    WrongType(Type, Type),
    WrongArithmeticOperands,
    FunctionNotDefined(FunctionSignature),
    ExpectedNumberOfOperands(usize),
    ParameterCannotBeVoid
}

pub type VerifyResult<T> = Result<T, VerifyError>;

pub fn create_verified_function(binder: &Binder, mut function: Function) -> Function {
    let mut verifier = Verifier::new(binder, &mut function);
    verifier.verify().unwrap();
    function
}

pub struct Verifier<'a> {
    function: &'a mut Function,
    binder: &'a Binder,
    operand_stack: Vec<Type>
}

impl<'a> Verifier<'a> {
    pub fn new(binder: &'a Binder, function: &'a mut Function) -> Verifier<'a> {
        Verifier {
            function,
            operand_stack: Vec::new(),
            binder
        }
    }

    pub fn verify(&mut self) -> VerifyResult<()> {
        for parameter in self.function.definition().parameters() {
            if parameter == &Type::Void {
                return Err(VerifyError::ParameterCannotBeVoid);
            }
        }

        self.verify_instructions()?;

        if !self.operand_stack.is_empty() {
            return Err(VerifyError::NonEmptyOperandStackOnReturn);
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
                    self.operand_stack.push(Type::Int32);
                }
                Instruction::LoadFloat32(_) => {
                    self.operand_stack.push(Type::Float32);
                }
                Instruction::LoadLocal(index) => {
                    let local_type = self.function.locals().get(*index as usize).ok_or(VerifyError::LocalIndexOutOfRange)?;
                    self.operand_stack.push(local_type.clone());
                }
                Instruction::StoreLocal(index) => {
                    let operand = self.pop_operand_stack()?;
                    let local_type = self.function.locals().get(*index as usize).ok_or(VerifyError::LocalIndexOutOfRange)?;
                    self.same_type(local_type, &operand)?;
                }
                Instruction::Add | Instruction::Sub => {
                    let op2 = self.pop_operand_stack()?;
                    let op1 = self.pop_operand_stack()?;
                    match (&op1, &op2) {
                        (Type::Int32, Type::Int32) => {
                            self.operand_stack.push(op1);
                        }
                        (Type::Float32, Type::Float32) => {
                            self.operand_stack.push(op1);
                        }
                        _ => {
                            return Err(VerifyError::WrongArithmeticOperands);
                        }
                    }
                }
                Instruction::Call(signature) => {
                    let func_to_call = self.binder.get(signature).ok_or(VerifyError::FunctionNotDefined(signature.clone()))?;
                    if self.operand_stack.len() < func_to_call.parameters().len() {
                        return Err(VerifyError::ExpectedNumberOfOperands(func_to_call.parameters().len()));
                    }

                    for parameter in func_to_call.parameters().iter().rev() {
                        let operand = self.pop_operand_stack()?;
                        self.same_type(parameter, &operand)?;
                    }

                    if func_to_call.return_type() != &Type::Void {
                        self.operand_stack.push(func_to_call.return_type().clone());
                    }
                }
                Instruction::LoadArgument(index) => {
                    let argument_type = self.function.definition().parameters().get(*index as usize).ok_or(VerifyError::ArgumentIndexOutOfRange)?;
                    self.operand_stack.push(argument_type.clone());
                }
                Instruction::Return => {
                    if self.function.definition().return_type() != &Type::Void {
                        let operand = self.pop_operand_stack()?;
                        self.same_type(self.function.definition().return_type(), &operand)?;
                    }
                }
            }
        }

        self.function.set_operand_stack_size(max_stack_size);

        Ok(())
    }

    fn pop_operand_stack(&mut self) -> VerifyResult<Type> {
        self.operand_stack.pop().ok_or(VerifyError::EmptyOperandStack)
    }

    fn same_type(&self, expected: &Type, actual: &Type) -> VerifyResult<()> {
        if expected != actual {
            Err(VerifyError::WrongType(expected.clone(), actual.clone()))
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
    assert_eq!(Err(VerifyError::NonEmptyOperandStackOnReturn), verifier.verify());
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
    assert_eq!(Err(VerifyError::NonEmptyOperandStackOnReturn), verifier.verify());
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
    assert_eq!(Err(VerifyError::WrongType(Type::Float32, Type::Int32)), verifier.verify());
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
    assert_eq!(Err(VerifyError::LocalIndexOutOfRange), verifier.verify());
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
    assert_eq!(Err(VerifyError::ExpectedNumberOfOperands(5)), verifier.verify());
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
    assert_eq!(Err(VerifyError::WrongType(Type::Float32, Type::Int32)), verifier.verify());
}