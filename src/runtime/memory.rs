use crate::runtime::heap::Heap;
use crate::model::typesystem::{Type, TypeStorage, TypeHolder};
use crate::runtime::{array, object};
use crate::model::class::{Class, ClassProvider};
use crate::runtime::object::{ObjectReference, ObjectHeader};
use crate::runtime::gc::GarbageCollector;

pub type ObjectPointer = *mut std::ffi::c_void;

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

            (*(obj_ptr as *mut ObjectHeader)).object_type = type_holder as *const TypeHolder;
        }

        // The header is skipped to make usage of objects easier & faster in code generator
        unsafe { obj_ptr.add(object::HEADER_SIZE) }
    }

    pub fn print_objects(&self) {
        for object_ref in HeapObjectsIterator::new(&self.heap) {
            println!("0x{:0x} - type: {}, size: {}", object_ref.ptr() as u64, object_ref.object_type().instance, object_ref.size());
        }
    }
}

pub struct HeapObjectsIterator<'a> {
    heap: &'a Heap,
    current_object_offset: usize
}

impl<'a> HeapObjectsIterator<'a> {
    pub fn new(heap: &'a Heap) -> HeapObjectsIterator<'a> {
        HeapObjectsIterator {
            heap,
            current_object_offset: 0
        }
    }
}

impl<'a> Iterator for HeapObjectsIterator<'a> {
    type Item = ObjectReference<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_object_offset >= self.heap.offset() {
            return None;
        }

        while self.current_object_offset < self.heap.offset() {
            match ObjectReference::from_full_ptr(unsafe { self.heap.data().as_ptr().add(self.current_object_offset) }) {
                Ok(object_ref) => {
                    self.current_object_offset += object_ref.full_size();
                    return Some(object_ref)
                }
                Err(deleted_size) => {
                    self.current_object_offset += deleted_size;
                }
            }
        }

        return None;
    }
}