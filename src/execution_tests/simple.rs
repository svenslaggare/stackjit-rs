use crate::model::function::{Function, FunctionDeclaration};
use crate::model::instruction::Instruction;
use crate::model::typesystem::TypeId;
use crate::vm::VirtualMachine;

#[test]
fn test1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}

#[test]
fn test2() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
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
fn test3() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
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
fn test4() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(10),
            Instruction::LoadInt32(20),
            Instruction::LoadInt32(30),
            Instruction::Add,
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(10 + 20 + 30, execution_result);
}

#[test]
fn test5() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(10),
            Instruction::LoadInt32(20),
            Instruction::Add,
            Instruction::LoadInt32(30),
            Instruction::Add,
            Instruction::LoadInt32(40),
            Instruction::Add,
            Instruction::LoadInt32(50),
            Instruction::Add,
            Instruction::LoadInt32(60),
            Instruction::Add,
            Instruction::LoadInt32(70),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(10 + 20 + 30 + 40 + 50 + 60 + 70, execution_result);
}

#[test]
fn test6() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(-10),
            Instruction::LoadInt32(-30),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(-40, execution_result);
}

#[test]
fn test7() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(-10),
            Instruction::LoadInt32(40),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(30, execution_result);
}

#[test]
fn test_locals1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
}

#[test]
fn test_locals2() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337, execution_result);
}

#[test]
fn test_locals3() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337, execution_result);
}

#[test]
fn test_locals4() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337 + 4711, execution_result);
}

#[test]
fn test_locals5() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(3000),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000 + 2000 + 3000, execution_result);
}

#[test]
fn test_locals6() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(3000),
            Instruction::Add,
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000 + 2000 + 3000, execution_result);
}

#[test]
fn test_locals7() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(3000),
            Instruction::Add,
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000 + 2000 + 3000, execution_result);
}

#[test]
fn test_locals8() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(1),

            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,

            Instruction::LoadLocal(0),
            Instruction::Add,

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337 + 4711 + 1337, execution_result);
}

#[test]
fn test_locals9() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32, TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(1),

            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,

            Instruction::LoadLocal(1),
            Instruction::Add,

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337 + 4711 + 4711, execution_result);
}

#[test]
fn test_locals10() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDeclaration::new_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadLocal(0),
            Instruction::Add,

            Instruction::LoadLocal(0),
            Instruction::Add,

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337 + 1337 + 1337, execution_result);
}