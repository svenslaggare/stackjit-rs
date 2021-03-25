use crate::vm::get_vm;
use crate::model::typesystem::{Type, TypeHolder};
use crate::engine::execution::RuntimeError;
use crate::model::function::{FunctionSignature, Function};
use crate::compiler::stack_layout;
use crate::runtime::stack_walker::StackFrame;

pub extern "C" fn set_error_return(return_address: u64, base_pointer: u64, stack_pointer: u64) {
    get_vm(|vm| {
        vm.engine.runtime_error.return_address = return_address;
        vm.engine.runtime_error.base_pointer = base_pointer;
        vm.engine.runtime_error.stack_pointer = stack_pointer;
    })
}

pub extern "C" fn new_array(type_ptr: u64, length: i32) -> *mut std::ffi::c_void {
    get_vm(|vm| {
        let type_holder = unsafe { (type_ptr as *const std::ffi::c_void as *const TypeHolder).as_ref() }.unwrap();
        vm.memory_manager.new_array(type_holder, length)
    })
}

pub extern "C" fn new_class(type_ptr: u64) -> *mut std::ffi::c_void {
    get_vm(|vm| {
        let type_holder = unsafe { (type_ptr as *const std::ffi::c_void as *const TypeHolder).as_ref() }.unwrap();
        let class = vm.engine.get_class(type_holder.instance.class_name().unwrap()).unwrap();
        vm.memory_manager.new_class(type_holder, class)
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

pub extern "C" fn print_stack_frame(base_pointer: u64, function_ptr: u64, instruction_index: usize) {
    get_vm(|vm| {
        let function = unsafe { (function_ptr as *const Function).as_ref().unwrap() };

        let compiled_function = vm.engine.compiler()
            .get_compiled_function(&function.definition().call_signature())
            .unwrap();

        println!("--------------------------------------------");

        let stack_frame = StackFrame::new(base_pointer, instruction_index, function, compiled_function);

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

pub extern "C" fn garbage_collect(base_pointer: u64, function_ptr: u64, instruction_index: usize) {
    get_vm(|vm| {
        let function = unsafe { (function_ptr as *const Function).as_ref().unwrap() };

        let compiled_function = vm.engine.compiler()
            .get_compiled_function(&function.definition().call_signature())
            .unwrap();

        let stack_frame = StackFrame::new(base_pointer, instruction_index, function, compiled_function);
        vm.memory_manager.garbage_collector.collect(
            vm.engine.compiler(),
            vm.engine.binder(),
            vm.engine.class_provider(),
            &mut vm.memory_manager.heap,
            stack_frame
        );
    });
}