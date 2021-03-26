use crate::vm::get_vm;
use crate::model::typesystem::{Type, TypeMetadata};
use crate::engine::execution::RuntimeError;
use crate::model::function::{FunctionSignature, Function};
use crate::compiler::stack_layout;
use crate::runtime::stack_walker::StackFrame;
use crate::runtime::memory::manager::ObjectPointer;

pub extern "C" fn set_error_return(return_address: u64, base_pointer: u64, stack_pointer: u64) {
    get_vm(|vm| {
        vm.engine.runtime_error.return_address = return_address;
        vm.engine.runtime_error.base_pointer = base_pointer;
        vm.engine.runtime_error.stack_pointer = stack_pointer;
    })
}

pub extern "C" fn new_array(type_ptr: *const TypeMetadata, length: i32) -> ObjectPointer {
    get_vm(|vm| {
        let type_metadata = unsafe { type_ptr.as_ref() }.unwrap();
        vm.memory_manager.new_array(type_metadata, length)
    })
}

pub extern "C" fn new_class(type_ptr: *const TypeMetadata) -> ObjectPointer {
    get_vm(|vm| {
        let type_metadata = unsafe { type_ptr.as_ref() }.unwrap();
        vm.memory_manager.new_class(type_metadata)
    })
}

pub extern "C" fn null_error(result_ptr: *mut u64) {
    runtime_error(result_ptr, RuntimeError::NullReference)
}

pub extern "C" fn array_create_error(result_ptr: *mut u64) {
    runtime_error(result_ptr, RuntimeError::ArrayCreate)
}

pub extern "C" fn array_bounds_error(result_ptr: *mut u64) {
    runtime_error(result_ptr, RuntimeError::ArrayBounds)
}

fn runtime_error(result_ptr: *mut u64, runtime_error: RuntimeError) {
    get_vm(|vm| {
        vm.engine.runtime_error.has_error = Some(runtime_error.clone());

        unsafe {
            *result_ptr = vm.engine.runtime_error.return_address;
            *result_ptr.add(1) = vm.engine.runtime_error.base_pointer;
            *result_ptr.add(2) = vm.engine.runtime_error.stack_pointer;
        }
    });
}

pub extern "C" fn print_stack_frame(base_pointer: u64, function_ptr: *const Function, instruction_index: usize) {
    get_vm(|vm| {
        let function = unsafe { function_ptr.as_ref().unwrap() };
        let compilation_data = vm.engine.compiler()
            .get_compilation_data(&function.definition().call_signature())
            .unwrap();

        println!("--------------------------------------------");

        let stack_frame = StackFrame::new(base_pointer, instruction_index, function, compilation_data);

        stack_frame.walk(
            vm.engine.compiler(),
            vm.engine.binder(),
            |frame| {
                frame.print_frame();
                println!();
            }
        );

        println!("--------------------------------------------");
    });
}

pub extern "C" fn garbage_collect(base_pointer: u64, function_ptr: *const Function, instruction_index: usize) {
    get_vm(|vm| {
        let function = unsafe { function_ptr.as_ref().unwrap() };
        let compilation_data = vm.engine.compiler()
            .get_compilation_data(&function.definition().call_signature())
            .unwrap();

        let stack_frame = StackFrame::new(base_pointer, instruction_index, function, compilation_data);
        vm.memory_manager.garbage_collector.collect(
            vm.engine.compiler(),
            vm.engine.binder(),
            &mut vm.memory_manager.heap,
            stack_frame
        );
    });
}