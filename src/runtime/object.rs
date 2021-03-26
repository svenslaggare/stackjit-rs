use crate::model::typesystem::{TypeStorage, Type, TypeMetadata};
use crate::runtime::memory::manager::ObjectPointer;
use crate::runtime::array;

#[repr(packed, C)]
#[derive(Copy, Clone)]
pub struct ObjectHeader {
    pub object_type: *const TypeMetadata,
    pub gc_info: u8
}

impl ObjectHeader {
    pub fn is_marked(&self) -> bool {
        self.get_gc_info().0
    }

    pub fn mark(&mut self) {
        self.set_gc_info(true, self.get_gc_info().1);
    }

    pub fn unmark(&mut self) {
        self.set_gc_info(false, self.get_gc_info().1);
    }

    pub fn increase_survival_count(&mut self) {
        let (marked, count) = self.get_gc_info();
        self.set_gc_info(marked, count + 1);
    }

    pub fn survival_count(&self) -> u8 {
        self.get_gc_info().1
    }

    fn delete(&mut self, size: u64) {
        unsafe {
            let object_type: &u64 = std::mem::transmute(&self.object_type);
            *(object_type as *const u64 as *mut u64) = size;
        }
        self.gc_info = 0xFF;
    }

    pub fn is_deleted(&self) -> bool {
        self.gc_info == 0xFF
    }

    pub fn deleted_size(&self) -> usize {
        (unsafe { self.object_type as u64 }) as usize
    }

    fn set_gc_info(&mut self, marked: bool, survived: u8) {
        self.gc_info = marked as u8 | ((survived & 0x7F) << 1);
    }

    fn get_gc_info(&self) -> (bool, u8) {
        let marked = (self.gc_info & 0x1) != 0;
        let survived = (self.gc_info >> 1) & 0x7F;
        (marked, survived)
    }
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ObjectHeader>();

pub struct ObjectReference<'a> {
    ptr: ObjectPointer,
    object_type: &'a TypeMetadata,
    size: usize
}

impl<'a> ObjectReference<'a> {
    pub fn from_full_ptr(ptr: *const u8) -> Result<ObjectReference<'a>, usize> {
        let object_header = ptr as *const ObjectHeader;

        unsafe {
            if (*object_header).is_deleted() {
                return Err((*object_header).deleted_size());
            }
        }

        let type_metadata = unsafe { (*object_header).object_type.as_ref() }.unwrap();

        let object_ptr = unsafe { ptr.add(HEADER_SIZE) };

        let object_size = match &type_metadata.instance {
            Type::Array(element) => {
                array::LENGTH_SIZE + element.size() * array::get_length(object_ptr as ObjectPointer)
            }
            Type::Class(_) => {
                type_metadata.class.as_ref().unwrap().memory_size()
            }
            _ => 0
        };

        Ok(
            ObjectReference {
                ptr: object_ptr as ObjectPointer,
                object_type: type_metadata,
                size: object_size
            }
        )
    }

    pub fn from_ptr(ptr: *const u8) -> Result<ObjectReference<'a>, usize> {
        ObjectReference::from_full_ptr(unsafe { ptr.sub(HEADER_SIZE) })
    }

    pub fn ptr(&self) -> ObjectPointer {
        self.ptr
    }

    pub fn full_ptr(&self) -> ObjectPointer {
        unsafe { self.ptr.sub(HEADER_SIZE) }
    }

    pub fn object_type(&self) -> &TypeMetadata {
        &self.object_type
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn full_size(&self) -> usize {
        self.size + HEADER_SIZE
    }

    pub fn header(&self) -> &ObjectHeader {
        unsafe { (self.full_ptr() as *const ObjectHeader).as_ref() }.unwrap()
    }

    pub fn header_mut(&mut self) -> &mut ObjectHeader {
        unsafe { (self.full_ptr() as *mut ObjectHeader).as_mut() }.unwrap()
    }

    pub fn delete(&mut self) {
        let full_size = self.full_size();
        self.header_mut().delete(full_size as u64);
    }
}

#[test]
fn test_gc_info1() {
    let mut header = ObjectHeader { object_type: std::ptr::null(), gc_info: 0 };
    assert!(!header.is_marked());

    header.mark();
    assert!(header.is_marked());
}

#[test]
fn test_gc_info2() {
    for marked in &[true, false] {
        for count in 0..127 {
            let mut header = ObjectHeader { object_type: std::ptr::null(), gc_info: 0 };
            header.set_gc_info(*marked, count);
            assert_eq!(*marked, header.is_marked());
            assert_eq!(count, header.survival_count());
        }
    }
}

#[test]
fn test_delete1() {
    let mut header = ObjectHeader { object_type: 0x13374711 as *const TypeMetadata, gc_info: 0 };
    assert!(!header.is_deleted());
    assert_eq!(0x13374711, header.deleted_size());

    header.delete(24251);

    assert!(header.is_deleted());
    assert_eq!(24251, header.deleted_size());
}