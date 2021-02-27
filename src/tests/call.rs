use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::engine::ExecutionEngine;

extern "C" fn sum(x: i32, y: i32) -> i32 {
    return x + y;
}

extern "C" fn sum8(x0: i32, x1: i32, x2: i32, x3: i32, x4: i32, x5: i32, x6: i32, x7: i32) -> i32 {
    return x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7;
}

extern "C" fn sub(x: i32, y: i32) -> i32 {
    return x - y;
}

#[test]
fn external1() {
    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "sum".to_owned(), vec![Type::Int32, Type::Int32], Type::Int32,
            sum as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature { name: "sum".to_owned(), parameters: vec![Type::Int32, Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337 + 4711, execution_result);
}

#[test]
fn external2() {
    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "sub".to_owned(), vec![Type::Int32, Type::Int32], Type::Int32,
            sub as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature { name: "sub".to_owned(), parameters: vec![Type::Int32, Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(4711 - 1337, execution_result);
}

#[test]
fn external3() {
    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "sum8".to_owned(), (0..8).map(|_| Type::Int32).collect(), Type::Int32,
            sum8 as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::LoadInt32(4),
            Instruction::LoadInt32(5),
            Instruction::LoadInt32(6),
            Instruction::LoadInt32(7),
            Instruction::LoadInt32(8),
            Instruction::Call(FunctionSignature::new("sum8".to_owned(), (0..8).map(|_| Type::Int32).collect())),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(36, execution_result);
}

#[test]
fn managed1() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum".to_owned(), vec![Type::Int32, Type::Int32], Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::LoadArgument(1),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature { name: "sum".to_owned(), parameters: vec![Type::Int32, Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337 + 4711, execution_result);
}

#[test]
fn managed2() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum".to_owned(), vec![Type::Int32, Type::Int32], Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadArgument(0),
            Instruction::StoreLocal(0),
            Instruction::LoadArgument(1),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature { name: "sum".to_owned(), parameters: vec![Type::Int32, Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337 + 4711, execution_result);
}

#[test]
fn managed3() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum8".to_owned(), (0..8).map(|_| Type::Int32).collect(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::LoadArgument(1),
            Instruction::Add,
            Instruction::LoadArgument(2),
            Instruction::Add,
            Instruction::LoadArgument(3),
            Instruction::Add,
            Instruction::LoadArgument(4),
            Instruction::Add,
            Instruction::LoadArgument(5),
            Instruction::Add,
            Instruction::LoadArgument(6),
            Instruction::Add,
            Instruction::LoadArgument(7),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::LoadInt32(4),
            Instruction::LoadInt32(5),
            Instruction::LoadInt32(6),
            Instruction::LoadInt32(7),
            Instruction::LoadInt32(8),
            Instruction::Call(FunctionSignature::new("sum8".to_owned(), (0..8).map(|_| Type::Int32).collect())),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(36, execution_result);
}

#[test]
fn managed4() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum7".to_owned(), (0..7).map(|_| Type::Int32).collect(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::LoadArgument(1),
            Instruction::Add,
            Instruction::LoadArgument(2),
            Instruction::Add,
            Instruction::LoadArgument(3),
            Instruction::Add,
            Instruction::LoadArgument(4),
            Instruction::Add,
            Instruction::LoadArgument(5),
            Instruction::Add,
            Instruction::LoadArgument(6),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum9".to_owned(), (0..9).map(|_| Type::Int32).collect(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::LoadArgument(1),
            Instruction::Add,
            Instruction::LoadArgument(2),
            Instruction::Add,
            Instruction::LoadArgument(3),
            Instruction::Add,
            Instruction::LoadArgument(4),
            Instruction::Add,
            Instruction::LoadArgument(5),
            Instruction::Add,
            Instruction::LoadArgument(6),
            Instruction::Add,
            Instruction::LoadArgument(7),
            Instruction::Add,
            Instruction::LoadArgument(8),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::LoadInt32(3),
            Instruction::LoadInt32(4),
            Instruction::LoadInt32(5),
            Instruction::LoadInt32(6),
            Instruction::LoadInt32(7),
            Instruction::LoadInt32(8),
            Instruction::LoadInt32(9),
            Instruction::Call(FunctionSignature::new("sum9".to_owned(), (0..9).map(|_| Type::Int32).collect())),
            Instruction::LoadInt32(11),
            Instruction::LoadInt32(12),
            Instruction::LoadInt32(13),
            Instruction::LoadInt32(14),
            Instruction::LoadInt32(15),
            Instruction::LoadInt32(16),
            Instruction::Call(FunctionSignature::new("sum7".to_owned(), (0..7).map(|_| Type::Int32).collect())),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16].iter().sum::<i32>(), execution_result);
}

#[test]
fn managed5() {
    let mut engine = ExecutionEngine::new();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature { name: "sum".to_owned(), parameters: vec![Type::Int32, Type::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum".to_owned(), vec![Type::Int32, Type::Int32], Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::LoadArgument(1),
            Instruction::Add,
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(1337 + 4711, execution_result);
}