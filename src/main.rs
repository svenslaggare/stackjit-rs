#[macro_use]
extern crate lazy_static;

mod model;
mod ir;
mod compiler;
mod engine;
mod tests;

use crate::compiler::jit::JitCompiler;
use crate::model::instruction::Instruction;
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::typesystem::Type;
use crate::model::verifier::create_verified_function;
use crate::engine::ExecutionEngine;

extern "C" fn push() -> i32 {
    return 4711;
}

extern "C" fn sum(x: i32, y: i32) -> i32 {
    return x + y;
}

extern "C" fn sum3(x: i32, y: i32, z: i32) -> i32 {
    return x + y + z;
}

extern "C" fn sum8(x0: i32, x1: i32, x2: i32, x3: i32, x4: i32, x5: i32, x6: i32, x7: i32) -> i32 {
    return x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7;
}

extern "C" fn print_float(x: f32) {
    println!("{}", x);
}

fn main() {
    let mut engine = ExecutionEngine::new();

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "sum8".to_owned(), (0..8).map(|_| Type::Int32).collect(), Type::Int32,
            sum8 as *mut libc::c_void
        )
    );

    engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut libc::c_void
        )
    );

    // jit_compiler.compile_function(&Function::new(
    //     FunctionDefinition::new_managed("sum8".to_owned(), (0..8).map(|_| Type::Int32).collect(), Type::Int32),
    //     Vec::new(),
    //     vec![
    //         Instruction::LoadArgument(0),
    //         Instruction::LoadArgument(1),
    //         Instruction::Add,
    //         Instruction::LoadArgument(2),
    //         Instruction::Add,
    //         Instruction::LoadArgument(3),
    //         Instruction::Add,
    //         Instruction::LoadArgument(4),
    //         Instruction::Add,
    //         Instruction::LoadArgument(5),
    //         Instruction::Add,
    //         Instruction::LoadArgument(6),
    //         Instruction::Add,
    //         Instruction::LoadArgument(7),
    //         Instruction::Add,
    //         Instruction::Return,
    //     ]
    // ));
    // println!();
    // println!();

    // let instructions = vec![
    //     Instruction::LoadInt32(1),
    //     Instruction::LoadInt32(2),
    //     Instruction::LoadInt32(3),
    //     Instruction::LoadInt32(4),
    //     Instruction::LoadInt32(5),
    //     Instruction::LoadInt32(6),
    //     Instruction::LoadInt32(7),
    //     Instruction::LoadInt32(8),
    //     Instruction::Call(FunctionSignature::new("sum8".to_owned(), (0..8).map(|_| Type::Int32).collect())),
    //     Instruction::Return,
    // ];
    //
    // let function = Function::new(
    //     FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
    //     // Vec::new(),
    //     vec![Type::Int32],
    //     instructions
    // );

    // jit_compiler.compile_function(&function);

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
    println!("Result: {}", execution_result);
}
