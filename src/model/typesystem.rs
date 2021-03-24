use std::collections::HashMap;
use std::ops::Deref;
use crate::model::class::Class;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Type {
    Void,
    Int32,
    Float32,
    Array(Box<Type>),
    Class(String)
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Int32 => 4,
            Type::Float32 => 4,
            Type::Array(_) => 8,
            Type::Class(_) => 8
        }
    }

    pub fn element_type(&self) -> Option<&Type> {
        if let Type::Array(element) = self {
            Some(element.deref())
        } else {
            None
        }
    }

    pub fn class_name(&self) -> Option<&str> {
        if let Type::Class(class) = self {
            Some(class)
        } else {
            None
        }
    }

    pub fn is_reference(&self) -> bool {
        match self {
            Type::Array(_) => true,
            Type::Class(_) => true,
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
        self == other
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => {
                write!(f, "Void")
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
            Type::Class(name) => {
                write!(f, "Ref.{}", name)
            }
        }
    }
}

pub struct TypeHolder {
    pub instance: Type,
    pub class_size: Option<usize>
}

pub struct TypeStorage {
    types: HashMap<Type, Box<TypeHolder>>,
}

impl TypeStorage {
    pub fn new() -> TypeStorage {
        TypeStorage {
            types: HashMap::new()
        }
    }

    pub fn add_class(&mut self, class: &Class) {
        let type_instance = Type::Class(class.name().to_owned());

        self.types.entry(type_instance.clone()).or_insert_with(|| {
            let type_holder = TypeHolder {
                instance: type_instance,
                class_size: Some(class.memory_size())
            };

            Box::new(type_holder)
        });
    }

    pub fn add_or_get_type(&mut self, type_instance: Type) -> &TypeHolder {
        self.types.entry(type_instance.clone()).or_insert_with(|| {
            let type_holder = TypeHolder {
                instance: type_instance,
                class_size: None
            };

            Box::new(type_holder)
        })
    }
}