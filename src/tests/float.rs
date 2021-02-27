use std::sync::{Arc, Mutex};

use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::compiler::jit::JitCompiler;
use crate::model::verifier::create_verified_function;
use crate::engine::ExecutionEngine;

lazy_static! {
   static ref FLOAT_RESULT: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
}

extern "C" fn print_float(x: f32) {
    println!("{}", x);
    *FLOAT_RESULT.lock().unwrap() = x;
}

extern "C" fn add(x: f32, y: f32) -> f32 {
    let result = x + y;
    *FLOAT_RESULT.lock().unwrap() = result;
    result
}

#[test]
fn test1() {
    *FLOAT_RESULT.lock().unwrap() = 0.0;

    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Float32],
        vec![
            Instruction::LoadFloat32(13.37),
            Instruction::LoadFloat32(47.11),
            Instruction::Add,
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![Type::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(0, execution_result);
    assert_eq!(13.37 + 47.11, *FLOAT_RESULT.lock().unwrap());
}

#[test]
fn test2() {
    *FLOAT_RESULT.lock().unwrap() = 0.0;

    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut libc::c_void
        )
    );

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "add".to_owned(), vec![Type::Float32, Type::Float32], Type::Float32,
            add as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Float32],
        vec![
            Instruction::LoadFloat32(13.37),
            Instruction::LoadFloat32(47.11),
            Instruction::Call(FunctionSignature { name: "add".to_owned(), parameters: vec![Type::Float32, Type::Float32] }),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![Type::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(0, execution_result);
    assert_eq!(13.37 + 47.11, *FLOAT_RESULT.lock().unwrap());
}

#[test]
fn test3() {
    *FLOAT_RESULT.lock().unwrap() = 0.0;

    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut libc::c_void
        )
    );

    engine.add_function(Function::new(
        FunctionDefinition::new_managed("sum8".to_owned(), (0..8).map(|_| Type::Float32).collect(), Type::Float32),
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
            Instruction::LoadFloat32(1.1),
            Instruction::LoadFloat32(2.1),
            Instruction::LoadFloat32(3.1),
            Instruction::LoadFloat32(4.1),
            Instruction::LoadFloat32(5.1),
            Instruction::LoadFloat32(6.1),
            Instruction::LoadFloat32(7.1),
            Instruction::LoadFloat32(8.1),
            Instruction::Call(FunctionSignature::new("sum8".to_owned(), (0..8).map(|_| Type::Float32).collect())),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![Type::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let function_ptr = engine.prepare_execution().unwrap();
    let execution_result = (function_ptr)();
    assert_eq!(0, execution_result);
    assert_eq!(1.1 + 2.1 + 3.1 + 4.1 + 5.1 + 6.1 + 7.1 + 8.1, *FLOAT_RESULT.lock().unwrap());
}