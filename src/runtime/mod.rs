pub mod heap;

use crate::runtime::heap::Heap;
use crate::model::typesystem::Type;

pub struct Runtime {
    heap: Heap
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime {
            heap: Heap::new(8 * 1024 * 1024)
        }
    }

    pub fn new_array(&mut self, type_instance: &Type, length: i32) -> *mut std::ffi::c_void {
        if let Type::Array(element) = type_instance {
            let array_size = 4 + length as usize * element.size();
            let obj_ptr = self.heap.allocate(array_size).unwrap();
            unsafe {
                *(obj_ptr as *mut i32) = length;
            }

            println!("Allocated array (type: {}, length: {}, size: {}): 0x{:x}", type_instance, length, array_size, obj_ptr as u64);
            obj_ptr
        } else {
            panic!("unexpected type.");
        }
    }
}

pub mod runtime_interface {
    use crate::vm::get_vm;
    use crate::model::typesystem::TypeId;

    pub extern "C" fn new_array(type_id: i32, length: i32) -> *mut std::ffi::c_void {
        get_vm(|vm| {
            let type_instance = vm.type_storage.get_type(TypeId(type_id)).unwrap();
            vm.runtime.new_array(type_instance, length)
        })
    }
}