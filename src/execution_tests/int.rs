use crate::model::function::{Function, FunctionDeclaration};
use crate::model::instruction::Instruction;
use crate::model::typesystem::TypeId;
use crate::vm::VirtualMachine;

#[test]
fn test1() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711 + 1337, execution_result);
}

#[test]
fn test2() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Sub,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711 - 1337, execution_result);
}

#[test]
fn test3() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Multiply,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711 * 1337, execution_result);
}

#[test]
fn test4() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Divide,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711 / 1337, execution_result);
}

#[test]
fn test5() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::LoadLocal(0),
            Instruction::Divide,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711 / 1337, execution_result);
}

