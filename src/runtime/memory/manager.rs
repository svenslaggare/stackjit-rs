use crate::model::class::Class;
use crate::model::typesystem::{Type, TypeId, TypeStorage};
use crate::runtime::array;
use crate::runtime::memory::gc::GarbageCollector;
use crate::runtime::memory::heap::Heap;
use crate::runtime::object::{ObjectHeader, ObjectPointer, ObjectReference};
use crate::runtime::object;

pub struct MemoryManager {
    pub heap: Heap,
    pub garbage_collector: GarbageCollector
}

impl MemoryManager {
    pub fn new() -> MemoryManager {
        MemoryManager {
            heap: Heap::new(8 * 1024 * 1024),
            garbage_collector: GarbageCollector::new()
        }
    }

    pub fn is_owned(&self, address: *const std::ffi::c_void) -> bool {
        self.heap.inside(address)
    }

    pub fn new_array(&mut self, type_instance: &Type, length: i32) -> ObjectPointer {
        let element_type = type_instance.id.element_type().expect("unexpected type");

        let array_size = array::LENGTH_SIZE + length as usize * element_type.size();
        let obj_ptr = self.new_object(type_instance, array_size);

        unsafe {
            *(obj_ptr as *mut i32) = length;
        }

        println!("Allocated array (type: {}, length: {}, size: {}): 0x{:x}", type_instance.id, length, array_size, obj_ptr as u64);
        obj_ptr
    }

    pub fn new_class(&mut self, type_instance: &Type) -> ObjectPointer {
        let obj_size = type_instance.class.as_ref().unwrap().memory_size();
        let obj_ptr = self.new_object(type_instance, obj_size);
        println!("Allocated class (type: {}, size: {}): 0x{:x}", type_instance.id, obj_size, obj_ptr as u64);
        obj_ptr
    }

    fn new_object(&mut self, type_instance: &Type, size: usize) -> ObjectPointer {
        let obj_ptr = self.heap.allocate(size + object::HEADER_SIZE).unwrap();

        unsafe {
            let obj_ptr = obj_ptr as *mut u8;
            for i in 0..size as isize {
                *obj_ptr.offset(i) = 0;
            }

            (*(obj_ptr as *mut ObjectHeader)).object_type = type_instance as *const Type;
        }

        // The header is skipped to make usage of objects easier & faster in code generator
        unsafe { obj_ptr.add(object::HEADER_SIZE) }
    }
}
