use crate::model::function::{Function, FunctionDefinition};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::engine::ExecutionEngine;

#[test]
fn test1() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(4711, execution_result);
}

#[test]
fn test2() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(4711 + 1337, execution_result);
}

#[test]
fn test3() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Sub,
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(4711 - 1337, execution_result);
}

#[test]
fn locals1() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337, execution_result);
}

#[test]
fn locals2() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337 + 4711, execution_result);
}