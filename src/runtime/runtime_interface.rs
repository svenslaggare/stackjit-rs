use crate::vm::get_vm;
use crate::model::typesystem::TypeId;
use crate::engine::execution::RuntimeError;

pub extern "C" fn set_error_return(return_address: u64, base_pointer: u64, stack_pointer: u64) {
    get_vm(|vm| {
        vm.engine.runtime_error.return_address = return_address;
        vm.engine.runtime_error.base_pointer = base_pointer;
        vm.engine.runtime_error.stack_pointer = stack_pointer;
    })
}

pub extern "C" fn new_array(type_id: i32, length: i32) -> *mut std::ffi::c_void {
    get_vm(|vm| {
        let type_instance = vm.type_storage.get_type(TypeId(type_id)).unwrap();
        vm.memory_manager.new_array(type_instance, length)
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
