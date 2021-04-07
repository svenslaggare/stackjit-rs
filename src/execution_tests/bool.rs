use std::cell::RefCell;

use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::TypeId;
use crate::vm::VirtualMachine;
use crate::runtime::array::ArrayReference;
use crate::runtime::object::ObjectPointer;

extern "C" fn get_element(array_ref: ObjectPointer, index: i32) -> i32 {
    let array_ref = ArrayReference::<bool>::new(array_ref);
    unsafe { *array_ref.get_raw(index as usize) as i32 }
}

extern "C" fn convert_bool_to_int(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

#[test]
fn test_simple1() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadTrue,
            Instruction::LoadFalse,
            Instruction::BranchEqual(6),

            Instruction::LoadInt32(2000),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),

            Instruction::LoadInt32(1000),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(2000, execution_result);
}

#[test]
fn test_simple2() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadTrue,
            Instruction::LoadTrue,
            Instruction::BranchEqual(6),

            Instruction::LoadInt32(2000),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),

            Instruction::LoadInt32(1000),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000, execution_result);
}

#[test]
fn test_array1() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "get_element".to_owned(), vec![TypeId::Array(Box::new(TypeId::Bool)), TypeId::Int32], TypeId::Int32,
            get_element as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(TypeId::Bool),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadTrue,
            Instruction::StoreElement(TypeId::Bool),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadFalse,
            Instruction::StoreElement(TypeId::Bool),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::Call(FunctionSignature { name: "get_element".to_string(), parameters: vec![TypeId::Array(Box::new(TypeId::Bool)), TypeId::Int32] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1, execution_result);
}

#[test]
fn test_array2() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "convert_bool_to_int".to_owned(), vec![TypeId::Bool], TypeId::Int32,
            convert_bool_to_int as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(TypeId::Bool),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadTrue,
            Instruction::StoreElement(TypeId::Bool),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadFalse,
            Instruction::StoreElement(TypeId::Bool),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::LoadElement(TypeId::Bool),
            Instruction::Call(FunctionSignature { name: "convert_bool_to_int".to_string(), parameters: vec![TypeId::Bool] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1, execution_result);
}

#[test]
fn test_compare1() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "convert_bool_to_int".to_owned(), vec![TypeId::Bool], TypeId::Int32,
            convert_bool_to_int as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(1000),
            Instruction::CompareEqual,
            Instruction::Call(FunctionSignature { name: "convert_bool_to_int".to_string(), parameters: vec![TypeId::Bool] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1, execution_result);
}

#[test]
fn test_compare2() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "convert_bool_to_int".to_owned(), vec![TypeId::Bool], TypeId::Int32,
            convert_bool_to_int as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(1000),
            Instruction::CompareNotEqual,
            Instruction::Call(FunctionSignature { name: "convert_bool_to_int".to_string(), parameters: vec![TypeId::Bool] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);
}

#[test]
fn test_compare3() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "convert_bool_to_int".to_owned(), vec![TypeId::Bool], TypeId::Int32,
            convert_bool_to_int as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::LoadInt32(2000),
            Instruction::CompareNotEqual,
            Instruction::Call(FunctionSignature { name: "convert_bool_to_int".to_string(), parameters: vec![TypeId::Bool] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1, execution_result);
}

#[test]
fn test_compare4() {
    let mut vm = VirtualMachine::new();

    vm.engine.binder_mut().define(
        FunctionDeclaration::with_external(
            "convert_bool_to_int".to_owned(), vec![TypeId::Bool], TypeId::Int32,
            convert_bool_to_int as *mut std::ffi::c_void
        )
    );

    vm.add_function(Function::new(
        FunctionDeclaration::with_manager("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Bool))],
        vec![
            Instruction::LoadFloat32(1000.0),
            Instruction::LoadFloat32(1000.0),
            Instruction::CompareEqual,
            Instruction::Call(FunctionSignature { name: "convert_bool_to_int".to_string(), parameters: vec![TypeId::Bool] }),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1, execution_result);
}