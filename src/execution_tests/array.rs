use std::cell::RefCell;

use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::vm::{VirtualMachine, get_vm};
use crate::runtime::array;
use crate::engine::execution::{ExecutionEngineError, RuntimeError};

thread_local!(static ARRAY_RESULT: RefCell<u64> = RefCell::new(0));
thread_local!(static FLOAT_RESULT: RefCell<f32> = RefCell::new(0.0));

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

extern "C" fn set_array_float(ptr: u64, index: i32, value: f32) {
    let ptr = (ptr + array::LENGTH_SIZE as u64) as *mut f32;
    unsafe {
        *ptr.add(index as usize) = value;
    }
}

extern "C" fn print_float(x: f32) {
    println!("{}", x);
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = x;
    });
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
        assert!(vm.memory_manager.is_owned(ARRAY_RESULT.with(|result| *result.borrow()) as *const std::ffi::c_void));
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
fn test_load2() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "set_array".to_owned(), vec![Type::Array(Box::new(Type::Float32)), Type::Int32, Type::Float32], Type::Void,
            set_array_float as *mut std::ffi::c_void
        )
    );

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Float32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Float32),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadFloat32(1337.0),
            Instruction::Call(FunctionSignature { name: "set_array".to_owned(), parameters: vec![Type::Array(Box::new(Type::Float32)), Type::Int32, Type::Float32] }),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Float32),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![Type::Float32] }),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1337.0, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test_load1_no_null_check() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
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
            Instruction::LoadInt32(146),
            Instruction::LoadInt32(4711),

            Instruction::StoreElement(Type::Int32),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(146),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return
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
            Instruction::LoadInt32(1),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1337, execution_result);
}

#[test]
fn test_store3() {
    FLOAT_RESULT.with(|result| {
        *result.borrow_mut() = 0.0;
    });

    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDefinition::new_external(
            "print".to_owned(), vec![Type::Float32], Type::Void,
            print_float as *mut std::ffi::c_void
        )
    );

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Float32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Float32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadFloat32(1337.0),
            Instruction::StoreElement(Type::Float32),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadFloat32(4711.0),
            Instruction::StoreElement(Type::Float32),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadElement(Type::Float32),
            Instruction::Call(FunctionSignature { name: "print".to_owned(), parameters: vec![Type::Float32] }),

            Instruction::LoadInt32(0),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
    assert_eq!(1337.0, FLOAT_RESULT.with(|result| *result.borrow()));
}

#[test]
fn test_load_length1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadArrayLength,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}

#[test]
fn test_load_length2() {
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
            Instruction::LoadInt32(-1),
            Instruction::StoreElement(Type::Int32),

            Instruction::LoadLocal(0),
            Instruction::LoadArrayLength,
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);
}

#[test]
fn test_checks1() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        Vec::new(),
        vec![
            Instruction::LoadNull,
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::NullReference)), execution_result);
}

#[test]
fn test_checks2() {
    let mut vm = VirtualMachine::new();

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

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::NullReference)), execution_result);
}

#[test]
fn test_checks3() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(1000),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::ArrayBounds)), execution_result);
}

#[test]
fn test_check4() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(-1),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::ArrayBounds)), execution_result);
}

#[test]
fn test_check5() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(-1),
            Instruction::LoadInt32(1337),
            Instruction::StoreElement(Type::Int32),
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::ArrayBounds)), execution_result);
}

#[test]
fn test_check6() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(-1),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Err(ExecutionEngineError::Runtime(RuntimeError::ArrayCreate)), execution_result);
}

#[test]
fn test_check7() {
    let mut vm = VirtualMachine::new();

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(0),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),
            Instruction::LoadInt32(4711),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute();
    assert_eq!(Ok(4711), execution_result);
}
