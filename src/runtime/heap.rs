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