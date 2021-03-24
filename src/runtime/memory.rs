use crate::runtime::heap::Heap;
use crate::model::typesystem::{Type, TypeStorage, TypeHolder};
use crate::runtime::{array, object};
use crate::model::class::{Class, ClassProvider};
use crate::runtime::object::ObjectReference;

pub type ObjectPointer = *mut std::ffi::c_void;

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

    pub fn new_array(&mut self, type_holder: &TypeHolder, length: i32) -> ObjectPointer {
        let element_type = type_holder.instance.element_type().expect("unexpected type");

        let array_size = array::LENGTH_SIZE + length as usize * element_type.size();
        let obj_ptr = self.new_object(type_holder, array_size);

        unsafe {
            *(obj_ptr as *mut i32) = length;
        }

        println!("Allocated array (type: {}, length: {}, size: {}): 0x{:x}", type_holder.instance, length, array_size, obj_ptr as u64);
        obj_ptr
    }

    pub fn new_class(&mut self, type_holder: &TypeHolder, class: &Class) -> ObjectPointer {
        let obj_size = class.memory_size();
        let obj_ptr = self.new_object(type_holder, obj_size);
        println!("Allocated class (type: {}, size: {}): 0x{:x}", type_holder.instance, obj_size, obj_ptr as u64);
        obj_ptr
    }

    fn new_object(&mut self, type_holder: &TypeHolder, size: usize) -> ObjectPointer {
        let obj_ptr = self.heap.allocate(size + object::HEADER_SIZE).unwrap();

        unsafe {
            let obj_ptr = obj_ptr as *mut u8;
            for i in 0..size as isize {
                *obj_ptr.offset(i) = 0;
            }

            *(obj_ptr as *mut u64) = type_holder as *const TypeHolder as *const u64 as u64
        }

        // The header is skipped to make usage of objects easier & faster in code generator
        unsafe { obj_ptr.add(object::HEADER_SIZE) }
    }

    pub fn print_objects(&self) {
        let heap = &self.heap;

        let data_ptr = heap.data().as_ptr();
        let mut current_object_offset = 0;
        while current_object_offset < heap.offset() {
            let object_ref = ObjectReference::from_ptr(unsafe { data_ptr.add(current_object_offset) });

            println!("0x{:0x} - type: {}, size: {}", object_ref.ptr() as u64, object_ref.object_type().instance, object_ref.size());
            current_object_offset += object_ref.full_size();
        }
    }
}