use crate::model::function::{FunctionDefinition, Function, FunctionSignature};
use crate::vm::VirtualMachine;
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::model::class::{Class, Field};

#[test]
fn test_stack_frame1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("create_int_array".to_owned(), vec![Type::Int32], Type::Array(Box::new(Type::Int32))),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::NewArray(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("create_array".to_owned(), vec![Type::Int32], Type::Array(Box::new(Type::Int32))),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            // Instruction::NewArray(Type::Int32),
            Instruction::Call(FunctionSignature { name: "create_int_array".to_string(), parameters: vec![Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32)), Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),

            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(1337),
            // Instruction::NewArray(Type::Int32),
            Instruction::Call(FunctionSignature { name: "create_array".to_string(), parameters: vec![Type::Int32] }),
            Instruction::StoreLocal(1),

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000, execution_result);
}

#[test]
fn test_collect1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), Type::Int32),
            Field::new("y".to_owned(), Type::Int32),
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32)), Type::Class("Point".to_owned())],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::StoreLocal(0),

            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(1),

            Instruction::Call(FunctionSignature { name: "std.gc.collect".to_string(), parameters: vec![] }),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
}