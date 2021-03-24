use std::cell::RefCell;

use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::vm::{VirtualMachine, get_vm};
use crate::runtime::array;
use crate::engine::execution::{ExecutionEngineError, RuntimeError};
use crate::model::class::{Class, Field};

thread_local!(static CLASS_RESULT: RefCell<u64> = RefCell::new(0));

extern "C" fn print_point(ptr: u64) {
    println!("0x{:x}", ptr);
    CLASS_RESULT.with(|result| {
        *result.borrow_mut() = ptr;
    });
}

extern "C" fn set_point_x(ptr: u64, value: i32) {
    unsafe {
        let ptr = ptr as *mut i32;
        *ptr = value;
    }
}

extern "C" fn print_array_element(ptr: u64, index: u64) {
    CLASS_RESULT.with(|result| {
        let class_ptr = unsafe { *((ptr + array::LENGTH_SIZE as u64) as *const u64).offset(index as isize) };
        println!("0x{:x}", class_ptr);
        *result.borrow_mut() = class_ptr;
    });
}

#[test]
fn test_create1() {
    CLASS_RESULT.with(|result| {
        *result.borrow_mut() = 0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print_point".to_owned(), vec![Type::Class("Point".to_owned())], Type::Void,
            print_point as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::NewObject("Point".to_owned()),
            Instruction::Call(FunctionSignature { name: "print_point".to_owned(), parameters: vec![Type::Class("Point".to_owned())] }),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    vm.engine.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), Type::Int32),
            Field::new("y".to_owned(), Type::Int32),
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    get_vm(|vm| {
        assert!(vm.memory_manager.is_owned(CLASS_RESULT.with(|result| *result.borrow()) as *const std::ffi::c_void));
    });
}

#[test]
fn test_load1() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "set_point_x".to_owned(), vec![Type::Class("Point".to_owned()), Type::Int32], Type::Void,
            set_point_x as *mut std::ffi::c_void
        )
    );

    vm.engine.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), Type::Int32),
            Field::new("y".to_owned(), Type::Int32),
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Class("Point".to_owned())],
        vec![
            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(i32::MIN),
            Instruction::Call(FunctionSignature { name: "set_point_x".to_owned(), parameters: vec![Type::Class("Point".to_owned()), Type::Int32] }),

            Instruction::LoadLocal(0),
            Instruction::LoadField("Point".to_owned(), "x".to_owned()),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(i32::MIN, execution_result);
}

#[test]
fn test_store1() {
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
        vec![Type::Class("Point".to_owned())],
        vec![
            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::StoreField("Point".to_owned(), "y".to_owned()),

            Instruction::LoadLocal(0),
            Instruction::LoadField("Point".to_owned(), "y".to_owned()),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}

#[test]
fn test_array1() {
    CLASS_RESULT.with(|result| {
        *result.borrow_mut() = 0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print_array_element".to_owned(), vec![Type::Array(Box::new(Type::Class("Point".to_owned()))), Type::Int32], Type::Void,
            print_array_element as *mut std::ffi::c_void
        )
    );

    vm.engine.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), Type::Int32),
            Field::new("y".to_owned(), Type::Int32),
        ]
    )).unwrap();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Class("Point".to_owned())))],
        vec![
            Instruction::LoadInt32(10),
            Instruction::NewArray(Type::Class("Point".to_owned())),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreElement(Type::Class("Point".to_owned())),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::Call(FunctionSignature {
                name: "print_array_element".to_string(),
                parameters: vec![Type::Array(Box::new(Type::Class("Point".to_owned()))), Type::Int32]
            }),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert!(vm.memory_manager.is_owned(CLASS_RESULT.with(|result| *result.borrow()) as *const std::ffi::c_void));
    });
}