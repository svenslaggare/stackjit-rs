use crate::model::typesystem::{TypeStorage, Type, TypeHolder};
use crate::runtime::memory::ObjectPointer;
use crate::model::class::ClassProvider;
use crate::runtime::array;

pub const HEADER_SIZE: usize = 8 + 1;

pub struct ObjectReference<'a> {
    ptr: ObjectPointer,
    object_type: &'a TypeHolder,
    size: usize
}

impl<'a> ObjectReference<'a> {
    pub fn from_ptr(ptr: *const u8) -> ObjectReference<'a> {
        let type_holder = unsafe {
            let type_holder = *(ptr as *const u64);
            (type_holder as *const TypeHolder).as_ref()
        }.unwrap();

        let object_ptr = unsafe { ptr.add(HEADER_SIZE) };

        let object_size = match &type_holder.instance {
            Type::Array(element) => {
                array::LENGTH_SIZE + element.size() * array::get_length(object_ptr as ObjectPointer)
            }
            Type::Class(_) => {
                type_holder.class_size.unwrap()
            }
            _ => 0
        };

        ObjectReference {
            ptr: object_ptr as ObjectPointer,
            object_type: type_holder,
            size: object_size
        }
    }

    pub fn ptr(&self) -> ObjectPointer {
        self.ptr
    }

    pub fn object_type(&self) -> &TypeHolder {
        &self.object_type
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn full_size(&self) -> usize {
        self.size + HEADER_SIZE
    }
}
