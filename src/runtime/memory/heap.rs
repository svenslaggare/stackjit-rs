use crate::model::typesystem::{TypeId, TypeStorage};
use crate::runtime::array;
use crate::runtime::object;
use crate::runtime::object::ObjectReference;

pub struct Heap {
    data: Vec<u8>,
    offset: usize
}

impl Heap {
    pub fn new(size: usize) -> Heap {
        Heap {
            data: vec![0; size],
            offset: 0
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn allocate(&mut self, size: usize) -> Option<*mut std::ffi::c_void> {
        if self.offset + size <= self.data.len() {
            let ptr = (&self.data[self.offset]) as *const u8 as *mut std::ffi::c_void;
            self.offset += size;
            Some(ptr)
        } else {
            None
        }
    }

    pub fn inside(&self, address: *const std::ffi::c_void) -> bool {
        let address = address as *const u8;
        address >= self.data.as_ptr() && address < (self.data.last().unwrap() as *const u8)
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