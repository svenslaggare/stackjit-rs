use crate::runtime::memory::ObjectPointer;

pub const LENGTH_SIZE: usize = 4;

pub fn get_length(ptr: ObjectPointer) -> usize {
    (unsafe { *(ptr as *const i32) }) as usize
}
