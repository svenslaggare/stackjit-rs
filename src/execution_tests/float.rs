use std::cell::RefCell;

use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::TypeId;
use crate::vm::VirtualMachine;

thread_local!(static FLOAT_RESULT: RefCell<f32> = RefCell::new(0.0));

extern "C" fn print_float(x: f32) {
    println!("{}", x);
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = x;
    });
}

extern "C" fn add(x: f32, y: f32) -> f32 {
    let result = x + y;
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = x;
    });
    result
}

#[test]
fn test1() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![],
        vec![
            Instruction::LoadFloat32(13.37),
            Instruction::LoadFloat32(47.11),
            Instruction::Add,
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(13.37 + 47.11, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test2() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "add".to_owned(), vec![TypeId::Float32, TypeId::Float32], TypeId::Float32,
            add as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![],
        vec![
            Instruction::LoadFloat32(13.37),
            Instruction::LoadFloat32(47.11),
            Instruction::Call(FunctionSignature { name: "add".to_owned(), parameters: vec![TypeId::Float32, TypeId::Float32] }),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(13.37 + 47.11, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test3() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("sum8".to_owned(), (0..8).map(|_| TypeId::Float32).collect(), TypeId::Float32),
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

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
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
            Instruction::Call(FunctionSignature::new("sum8".to_owned(), (0..8).map(|_| TypeId::Float32).collect())),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1.1 + 2.1 + 3.1 + 4.1 + 5.1 + 6.1 + 7.1 + 8.1, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test4() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![],
        vec![
            Instruction::LoadFloat32(13.37),
            Instruction::LoadFloat32(47.11),
            Instruction::Sub,
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(13.37 - 47.11, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test5() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Float32],
        vec![
            Instruction::LoadFloat32(1000.0),
            Instruction::LoadFloat32(2000.0),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadFloat32(3000.0),
            Instruction::LoadLocal(0),
            Instruction::Add,
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),

            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1000.0 + 2000.0 + 3000.0, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test6() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Float32],
        vec![
            Instruction::LoadFloat32(1000.0),
            Instruction::LoadFloat32(2000.0),
            Instruction::Add,
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadFloat32(3000.0),
            Instruction::Add,
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),

            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1000.0 + 2000.0 + 3000.0, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test7() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "print".to_owned(), vec![TypeId::Float32], TypeId::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Float32, TypeId::Float32],
        vec![
            Instruction::LoadFloat32(1337.0),
            Instruction::StoreLocal(0),
            Instruction::LoadFloat32(4711.0),
            Instruction::StoreLocal(1),

            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::Add,

            Instruction::LoadLocal(0),
            Instruction::Add,

            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![TypeId::Float32] }),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1337.0 + 4711.0 + 1337.0, FLOAT_RESULT.with(|result| *result.borrow()));
}