use crate::model::function::{FunctionDeclaration, Function, FunctionSignature};
use crate::vm::{VirtualMachine, get_vm};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{TypeId, Type};
use crate::model::class::{Class, Field};

#[test]
fn test_stack_frame1() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("create_int_array".to_owned(), vec![TypeId::Int32], TypeId::Array(Box::new(TypeId::Int32))),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::Call(FunctionSignature::new("std.gc.print_stack_frame".to_string(), vec![])),
            Instruction::NewArray(TypeId::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("create_array".to_owned(), vec![TypeId::Int32], TypeId::Array(Box::new(TypeId::Int32))),
        Vec::new(),
        vec![
            Instruction::LoadArgument(0),
            Instruction::Call(FunctionSignature::new("create_int_array".to_string(), vec![TypeId::Int32])),
            Instruction::Return,
        ]
    )).unwrap();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32)), TypeId::Array(Box::new(TypeId::Int32))],
        vec![
            Instruction::LoadInt32(1000),

            Instruction::LoadInt32(4711),
            Instruction::Call(FunctionSignature::new("std.gc.print_stack_frame".to_string(), vec![])),
            Instruction::NewArray(TypeId::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(1337),
            Instruction::Call(FunctionSignature::new("create_array".to_string(), vec![TypeId::Int32])),
            Instruction::StoreLocal(1),

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(1000, execution_result);
}

#[test]
fn test_collect1() {
    let mut vm = VirtualMachine::new();

    vm.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), TypeId::Int32),
            Field::new("y".to_owned(), TypeId::Int32),
        ]
    ));

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32)), TypeId::Class("Point".to_owned())],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadNull(TypeId::Array(Box::new(TypeId::Int32))),
            Instruction::StoreLocal(0),

            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(1),
            Instruction::LoadLocal(1),
            Instruction::LoadInt32(4711),
            Instruction::StoreField("Point".to_owned(), "x".to_owned()),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::LoadLocal(1),
            Instruction::LoadField("Point".to_owned(), "x".to_owned()),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);

    get_vm(|vm| {
        assert_eq!(1, vm.memory_manager.garbage_collector.deleted_objects().len());
        assert_eq!(TypeId::Array(Box::new(TypeId::Int32)), vm.memory_manager.garbage_collector.deleted_objects()[0].1);
    });
}

#[test]
fn test_collect2() {
    let mut vm = VirtualMachine::new();

    vm.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), TypeId::Int32),
            Field::new("y".to_owned(), TypeId::Int32),
        ]
    ));

    let point_type = TypeId::Class("Point".to_owned());

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32)), point_type.clone()],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(100),
            Instruction::LoadInt32(4711),
            Instruction::StoreElement(TypeId::Int32),

            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(1),

            Instruction::LoadNull(TypeId::Class("Point".to_owned())),
            Instruction::StoreLocal(1),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(100),
            Instruction::LoadElement(TypeId::Int32),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(4711, execution_result);

    get_vm(|vm| {
        assert_eq!(1, vm.memory_manager.garbage_collector.deleted_objects().len());
        assert_eq!(point_type.clone(), vm.memory_manager.garbage_collector.deleted_objects()[0].1);
    });
}


#[test]
fn test_collect3() {
    let mut vm = VirtualMachine::new();

    vm.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), TypeId::Int32),
            Field::new("y".to_owned(), TypeId::Int32),
        ]
    ));

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32)), TypeId::Class("Point".to_owned())],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),
            Instruction::StoreLocal(0),

            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreLocal(1),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(0, vm.memory_manager.garbage_collector.deleted_objects().len());
    });
}

#[test]
fn test_collect4() {
    let mut vm = VirtualMachine::new();

    vm.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), TypeId::Int32),
            Field::new("y".to_owned(), TypeId::Int32),
        ]
    ));

    let point_type = TypeId::Class("Point".to_owned());

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(point_type.clone()))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(point_type.clone()),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(100),
            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreElement(point_type.clone()),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(0, vm.memory_manager.garbage_collector.deleted_objects().len());
    });
}

#[test]
fn test_collect5() {
    let mut vm = VirtualMachine::new();

    vm.add_class(Class::new(
        "Point".to_owned(),
        vec![
            Field::new("x".to_owned(), TypeId::Int32),
            Field::new("y".to_owned(), TypeId::Int32),
        ]
    ));

    let point_type = TypeId::Class("Point".to_owned());

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(point_type.clone()))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(point_type.clone()),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(100),
            Instruction::NewObject("Point".to_owned()),
            Instruction::StoreElement(point_type.clone()),

            Instruction::LoadNull(TypeId::Array(Box::new(point_type.clone()))),
            Instruction::StoreLocal(0),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(2, vm.memory_manager.garbage_collector.deleted_objects().len());
        assert_eq!(TypeId::Array(Box::new(point_type.clone())), vm.memory_manager.garbage_collector.deleted_objects()[0].1);
        assert_eq!(point_type.clone(), vm.memory_manager.garbage_collector.deleted_objects()[1].1);
    });
}

#[test]
fn test_collect6() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32))],
        vec![
            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::StoreLocal(0),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(0, vm.memory_manager.garbage_collector.deleted_objects().len());
    });
}

#[test]
fn test_collect7() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Int32, TypeId::Array(Box::new(TypeId::Int32))],
        vec![
            Instruction::LoadLocal(0),

            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::StoreLocal(1),

            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(0, vm.memory_manager.garbage_collector.deleted_objects().len());
    });
}

#[test]
fn test_collect8() {
    let mut vm = VirtualMachine::new();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("inner".to_owned(), Vec::new(), TypeId::Array(Box::new(TypeId::Int32))),
        vec![TypeId::Array(Box::new(TypeId::Int32)), TypeId::Array(Box::new(TypeId::Int32))],
        vec![
            Instruction::LoadLocal(0),

            Instruction::LoadInt32(4711),
            Instruction::NewArray(TypeId::Int32),

            Instruction::Call(FunctionSignature::new("std.gc.collect".to_string(), vec![])),

            Instruction::StoreLocal(1),

            Instruction::Return,
        ]
    )).unwrap();

    vm.add_function(Function::new(
        FunctionDeclaration::with_managed("main".to_owned(), Vec::new(), TypeId::Int32),
        vec![TypeId::Array(Box::new(TypeId::Int32))],
        vec![
            Instruction::Call(FunctionSignature::new("inner".to_string(), vec![])),
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(0, execution_result);

    get_vm(|vm| {
        assert_eq!(0, vm.memory_manager.garbage_collector.deleted_objects().len());
    });
}