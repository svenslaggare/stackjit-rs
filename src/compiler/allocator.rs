pub struct ExecutableMemoryAllocator {
    pages: Vec<ExecutablePage>
}

impl ExecutableMemoryAllocator {
    pub fn new() -> ExecutableMemoryAllocator {
        ExecutableMemoryAllocator {
            pages: Vec::new()
        }
    }

    pub fn allocate(&mut self, size: usize) -> *mut std::ffi::c_void {
        for page in &mut self.pages {
            if let Some(address) = page.try_allocate(size) {
                return address;
            }
        }

        // No page with enough room, allocate new
        let page_size = 4096;
        let mut page = ExecutablePage::new(((size + page_size - 1) / page_size) * page_size).unwrap(); //Align to page size
        let address = page.try_allocate(size).unwrap();
        self.pages.push(page);
        address
    }
}

struct ExecutablePage {
    address: *mut std::ffi::c_void,
    size: usize,
    current_offset: usize
}

impl ExecutablePage {
    pub fn new(size: usize) -> Option<ExecutablePage> {
        let page_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_WRITE | libc::PROT_READ | libc::PROT_EXEC,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0
            )
        };

        if page_ptr != std::ptr::null_mut() {
            Some(
                ExecutablePage {
                    address: page_ptr,
                    size,
                    current_offset: 0
                }
            )
        } else {
            None
        }
    }

    pub fn try_allocate(&mut self, size: usize) -> Option<*mut std::ffi::c_void> {
        let size_left = self.size - self.current_offset;
        if size_left >= size {
            let ptr = unsafe { self.address.add(self.current_offset) };
            self.current_offset += size;
            Some(ptr)
        } else {
            None
        }
    }
}

impl Drop for ExecutablePage {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.address, self.size);
        }
    }
}

#[test]
fn test_allocate_page1() {
    let mut page = ExecutablePage::new(4096).unwrap();
    let allocate1 = page.try_allocate(100);
    assert!(allocate1.is_some());

    let allocate2 = page.try_allocate(100);
    assert!(allocate2.is_some());
    assert_ne!(allocate1, allocate2);

    let allocate3 = page.try_allocate(5000);
    assert!(allocate3.is_none());
}

#[test]
fn test_allocate_manager1() {
    let mut allocator = ExecutableMemoryAllocator::new();
    assert_ne!(std::ptr::null_mut(), allocator.allocate(100));
    assert_ne!(std::ptr::null_mut(), allocator.allocate(5100));
    assert_eq!(2, allocator.pages.len());
}