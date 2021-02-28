use crate::runtime::heap::Heap;
use crate::model::typesystem::Type;
use crate::runtime::array;

pub struct MemoryManager {
    heap: Heap
}

impl MemoryManager {
    pub fn new() -> MemoryManager {
        MemoryManager {
            heap: Heap::new(8 * 1024 * 1024)
        }
    }

    pub fn is_owned(&self, address: *const std::ffi::c_void) -> bool {
        self.heap.inside(address)
    }

    pub fn new_array(&mut self, type_instance: &Type, length: i32) -> *mut std::ffi::c_void {
        let element_type = type_instance.element_type().expect("unexpected type");

        let data_size = length as usize * element_type.size();
        let array_size = array::LENGTH_SIZE + data_size;
        let obj_ptr = self.heap.allocate(array_size).unwrap();
        unsafe {
            *(obj_ptr as *mut i32) = length;

            let data_ptr = obj_ptr.add(array::LENGTH_SIZE) as *mut u8;
            for i in 0..array_size as isize {
                *data_ptr.offset(i) = 0;
            }
        }

        println!("Allocated array (type: {}, length: {}, size: {}): 0x{:x}", type_instance, length, array_size, obj_ptr as u64);
        obj_ptr
    }
}