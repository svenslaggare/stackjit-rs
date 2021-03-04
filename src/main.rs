#[macro_use]
extern crate lazy_static;

mod model;
mod ir;
mod compiler;
mod runtime;
mod engine;
mod vm;
mod execution_tests;

use crate::model::instruction::Instruction;
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::typesystem::Type;
use crate::vm::VirtualMachine;

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

extern "C" fn print_array(ptr: u64) {
    println!("0x{:x}", ptr);
}

fn main() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "sum8".to_owned(), (0..8).map(|_| Type::Int32).collect(), Type::Int32,
            sum8 as *mut std::ffi::c_void
        )
    );

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print_array".to_owned(), vec![Type::Array(Box::new(Type::Int32))], Type::Void,
            print_array as *mut std::ffi::c_void
        )
    );

    // vm.engine.add_function(Function::new(
    //     FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
    //     vec![Type::Array(Box::new(Type::Int32))],
    //     vec![
    //         Instruction::LoadInt32(4000),
    //         Instruction::NewArray(Type::Int32),
    //         Instruction::Call(FunctionSignature { name: "print_array".to_owned(), parameters: vec![Type::Array(Box::new(Type::Int32))] }),
    //         Instruction::LoadInt32(0),
    //         Instruction::Return,
    //     ]
    // )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("new_array".to_owned(), Vec::new(), Type::Int32),
        vec![],
        vec![
            Instruction::LoadNull,
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(2000),
            Instruction::Call(FunctionSignature { name: "new_array".to_owned(), parameters: vec![] }),
            Instruction::Add,
            Instruction::Return
        ]
    )).unwrap();

    println!("Result: {:?}", vm.execute());
}
