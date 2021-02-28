use std::cell::RefCell;

use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::vm::{VirtualMachine, get_vm};
use crate::runtime::array;

thread_local!(static ARRAY_RESULT: RefCell<u64> = RefCell::new(0));

extern "C" fn print_array(ptr: u64) {
    println!("0x{:x}", ptr);
    ARRAY_RESULT.with(|result| {
        *result.borrow_mut() = ptr;
    });
}

extern "C" fn set_array(ptr: u64, index: i32, value: i32) {
    let ptr = (ptr + array::LENGTH_SIZE as u64) as *mut i32;
    unsafe {
        *ptr.add(index as usize) = value;
    }
}

#[test]
fn test_create1() {
    ARRAY_RESULT.with(|result| {
        *result.borrow_mut() = 0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print_array".to_owned(), vec![Type::Array(Box::new(Type::Int32))], Type::Void,
            print_array as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::Call(FunctionSignature { name: "print_array".to_owned(), parameters: vec![Type::Array(Box::new(Type::Int32))] }),
            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    get_vm(|vm| {
        assert!(vm.memory_manager.owned_by(ARRAY_RESULT.with(|result| *result.borrow()) as *const std::ffi::c_void));
    });
}

#[test]
fn test_load1() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "set_array".to_owned(), vec![Type::Array(Box::new(Type::Int32)), Type::Int32, Type::Int32], Type::Void,
            set_array as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadInt32(i32::MIN),
            Instruction::Call(FunctionSignature { name: "set_array".to_owned(), parameters: vec![Type::Array(Box::new(Type::Int32)), Type::Int32, Type::Int32] }),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(i32::MIN, execution_result);
}

#[test]
fn test_store1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadInt32(4711),

            Instruction::StoreElement(Type::Int32),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}

#[test]
fn test_store2() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(1337),
            Instruction::StoreElement(Type::Int32),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadInt32(4711),
            Instruction::StoreElement(Type::Int32),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}