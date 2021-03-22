use crate::runtime::heap::Heap;
use crate::model::typesystem::Type;
use crate::runtime::array;
use crate::model::class::Class;

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

        let array_size = array::LENGTH_SIZE + length as usize * element_type.size();
        let obj_ptr = self.new_object(array_size);

        unsafe {
            *(obj_ptr as *mut i32) = length;
        }

        println!("Allocated array (type: {}, length: {}, size: {}): 0x{:x}", type_instance, length, array_size, obj_ptr as u64);
        obj_ptr
    }

    pub fn new_class(&mut self, type_instance: &Type, class: &Class) -> *mut std::ffi::c_void {
        let obj_size = class.memory_size();
        let obj_ptr = self.new_object(obj_size);
        println!("Allocated class (type: {}, size: {}): 0x{:x}", type_instance, obj_size, obj_ptr as u64);
        obj_ptr
    }

    fn new_object(&mut self, size: usize) -> *mut std::ffi::c_void {
        let obj_ptr = self.heap.allocate(size).unwrap();

        unsafe {
            let obj_ptr = obj_ptr as *mut u8;
            for i in 0..size as isize {
                *obj_ptr.offset(i) = 0;
            }
        }

        obj_ptr
    }
}