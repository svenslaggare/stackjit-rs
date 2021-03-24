use crate::model::typesystem::{TypeStorage, Type, TypeId};
use crate::runtime::memory::ObjectPointer;
use crate::model::class::ClassProvider;
use crate::runtime::array;

pub const HEADER_SIZE: usize = 4 + 1;

pub struct ObjectReference<'a> {
    ptr: ObjectPointer,
    object_type: &'a Type,
    size: usize
}

impl<'a> ObjectReference<'a> {
    pub fn from_ptr(ptr: *const u8,
                    type_storage: &'a TypeStorage,
                    class_provider: &ClassProvider) -> ObjectReference<'a> {
        let type_id = unsafe { *(ptr as *const i32) };
        let type_instance = type_storage.get_type(TypeId(type_id)).unwrap();

        let object_ptr = unsafe { ptr.add(HEADER_SIZE) };

        let object_size = match type_instance {
            Type::Array(element) => {
                array::LENGTH_SIZE + element.size() * array::get_length(object_ptr as ObjectPointer)
            }
            Type::Class(name) => {
                class_provider.get(name).unwrap().memory_size()
            }
            _ => 0
        };

        ObjectReference {
            ptr: object_ptr as ObjectPointer,
            object_type: type_instance,
            size: object_size
        }
    }

    pub fn ptr(&self) -> ObjectPointer {
        self.ptr
    }

    pub fn object_type(&self) -> &Type {
        self.object_type
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn full_size(&self) -> usize {
        self.size + HEADER_SIZE
    }
}
