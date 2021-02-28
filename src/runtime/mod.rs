pub mod memory;
pub mod heap;
pub mod array;

pub mod runtime_interface {
    use crate::vm::get_vm;
    use crate::model::typesystem::TypeId;

    pub extern "C" fn new_array(type_id: i32, length: i32) -> *mut std::ffi::c_void {
        get_vm(|vm| {
            let type_instance = vm.type_storage.get_type(TypeId(type_id)).unwrap();
            vm.memory_manager.new_array(type_instance, length)
        })
    }
}