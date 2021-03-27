use std::collections::HashMap;
use std::ops::Deref;

use crate::model::class::Class;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeId {
    Void,
    Int32,
    Float32,
    Array(Box<TypeId>),
    Class(String)
}

impl TypeId {
    pub fn size(&self) -> usize {
        match self {
            TypeId::Void => 0,
            TypeId::Int32 => 4,
            TypeId::Float32 => 4,
            TypeId::Array(_) => 8,
            TypeId::Class(_) => 8
        }
    }

    pub fn element_type(&self) -> Option<&TypeId> {
        if let TypeId::Array(element) = self {
            Some(element.deref())
        } else {
            None
        }
    }

    pub fn class_name(&self) -> Option<&str> {
        if let TypeId::Class(class) = self {
            Some(class)
        } else {
            None
        }
    }

    pub fn is_reference(&self) -> bool {
        match self {
            TypeId::Array(_) => true,
            TypeId::Class(_) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            TypeId::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_class(&self) -> bool {
        match self {
            TypeId::Class(_) => true,
            _ => false,
        }
    }

    pub fn is_same_type(&self, other: &TypeId) -> bool {
        self == other
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeId::Void => {
                write!(f, "Void")
            }
            TypeId::Int32 => {
                write!(f, "Int32")
            }
            TypeId::Float32 => {
                write!(f, "Float32")
            }
            TypeId::Array(element) => {
                write!(f, "Ref.Array[{}]", element)
            }
            TypeId::Class(name) => {
                write!(f, "Ref.{}", name)
            }
        }
    }
}

pub struct Type {
    pub id: TypeId,
    pub class: Option<Class>
}

pub struct TypeStorage {
    types: HashMap<TypeId, Box<Type>>,
}

impl TypeStorage {
    pub fn new() -> TypeStorage {
        TypeStorage {
            types: HashMap::new()
        }
    }

    pub fn add_class(&mut self, class: Class) {
        let type_id = TypeId::Class(class.name().to_owned());

        self.types.entry(type_id.clone()).or_insert_with(|| {
            Box::new(
                Type {
                    id: type_id,
                    class: Some(class)
                }
            )
        });
    }

    pub fn get(&self, type_id: &TypeId) -> Option<&Type> {
        self.types.get(type_id).map(|t| t.as_ref())
    }

    pub fn entry(&mut self, type_id: TypeId) -> &Type {
        self.types.entry(type_id.clone()).or_insert_with(|| {
            assert!(!type_id.is_class());

            Box::new(
                Type {
                    id: type_id,
                    class: None
                }
            )
        })
    }
}