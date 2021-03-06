use std::collections::HashMap;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Null,
    Int32,
    Float32,
    Array(Box<Type>)
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Null => 0,
            Type::Int32 => 4,
            Type::Float32 => 4,
            Type::Array(_) => 8
        }
    }

    pub fn element_type(&self) -> Option<&Type> {
        if let Type::Array(element) = self {
            Some(element.deref())
        } else {
            None
        }
    }

    pub fn is_reference(&self) -> bool {
        match self {
            Type::Null | Type::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Type::Null => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Type::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_same_type(&self, other: &Type) -> bool {
        self == other || (self.is_reference() && other.is_null()) || (self.is_null() && other.is_reference())
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => {
                write!(f, "Void")
            }
            Type::Null => {
                write!(f, "Null")
            }
            Type::Int32 => {
                write!(f, "Int32")
            }
            Type::Float32 => {
                write!(f, "Float32")
            }
            Type::Array(element) => {
                write!(f, "Ref.Array[{}]", element)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(pub i32);

pub struct TypeStorage {
    types: Vec<Type>,
    type_to_id: HashMap<Type, TypeId>
}

impl TypeStorage {
    pub fn new() -> TypeStorage {
        let mut type_storage = TypeStorage {
            types: Vec::new(),
            type_to_id: HashMap::new(),
        };

        type_storage.add_or_get_type(Type::Void);
        type_storage.add_or_get_type(Type::Null);
        type_storage.add_or_get_type(Type::Int32);
        type_storage.add_or_get_type(Type::Float32);

        type_storage
    }

    pub fn add_or_get_type(&mut self, type_instance: Type) -> TypeId {
        match self.type_to_id.get(&type_instance) {
            Some(type_id) => *type_id,
            None => {
                let id = TypeId(self.types.len() as i32);
                self.types.push(type_instance.clone());
                self.type_to_id.insert(type_instance, id);
                id
            }
        }
    }

    pub fn get_type(&self, type_id: TypeId) -> Option<&Type> {
        self.types.get(type_id.0 as usize)
    }
}