use crate::runtime::object::ObjectPointer;

pub const LENGTH_SIZE: usize = 4;

pub fn get_length(ptr: ObjectPointer) -> usize {
    (unsafe { *(ptr as *const i32) }) as usize
}

pub fn get_elements<T>(ptr: ObjectPointer) -> *const T {
    unsafe { (ptr.add(LENGTH_SIZE)) as *const T }
}

pub struct ArrayReference<T> {
    elements_ptr: *const T,
    length: usize
}

impl<T> ArrayReference<T> {
    pub fn new(ptr: ObjectPointer) -> ArrayReference<T> {
        ArrayReference {
            elements_ptr: get_elements(ptr),
            length: get_length(ptr)
        }
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn get_raw(&self, index: usize) -> *const u8 {
        unsafe { self.elements_ptr.add(index) as *const u8 }
    }
}