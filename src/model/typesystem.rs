use std::collections::HashMap;
use std::ops::Deref;
use std::iter::FromIterator;

use crate::model::class::Class;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeId {
    Void,
    Int32,
    Float32,
    Bool,
    Array(Box<TypeId>),
    Class(String)
}

impl TypeId {
    pub fn size(&self) -> usize {
        match self {
            TypeId::Void => 0,
            TypeId::Int32 => 4,
            TypeId::Float32 => 4,
            TypeId::Bool => 1,
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

    pub fn is_float(&self) -> bool {
        match self {
            TypeId::Float32 => true,
            _ => false,
        }
    }

    pub fn is_same_type(&self, other: &TypeId) -> bool {
        self == other
    }

    pub fn from_str(text: &str) -> Option<TypeId> {
        TypeId::parse_type(&text.chars().collect::<Vec<_>>()[..])
    }

    fn parse_type(text: &[char]) -> Option<TypeId> {
        if text.is_empty() {
            return None;
        }

        let void_chars = TypeId::Void.to_string().chars().collect::<Vec<_>>();
        let int_chars = TypeId::Int32.to_string().chars().collect::<Vec<_>>();
        let float_chars = TypeId::Float32.to_string().chars().collect::<Vec<_>>();
        let bool_chars = TypeId::Bool.to_string().chars().collect::<Vec<_>>();

        let ref_array_chars = "Ref.Array[".chars().collect::<Vec<_>>();
        let ref_chars = "Ref.".chars().collect::<Vec<_>>();

        if text.starts_with(&void_chars[..]) {
            Some(TypeId::Void)
        } else if text.starts_with(&int_chars[..]) {
            Some(TypeId::Int32)
        } else if text.starts_with(&float_chars[..]) {
            Some(TypeId::Float32)
        } else if text.starts_with(&bool_chars[..]) {
            Some(TypeId::Bool)
        } else if text.starts_with(&ref_array_chars[..]) {
            let element_type = TypeId::parse_type(&text[ref_array_chars.len()..])?;
            Some(TypeId::Array(Box::new(element_type)))
        } else if text.starts_with(&ref_chars[..]) {
            let end = text.iter().position(|c| c == &']').unwrap_or(text.len());
            Some(TypeId::Class(String::from_iter(&text[ref_chars.len()..end])))
        } else {
            None
        }
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeId::Void => {
                write!(f, "Void")
            }
            TypeId::Int32 => {
                write!(f, "Int")
            }
            TypeId::Float32 => {
                write!(f, "Float")
            }
            TypeId::Bool => {
                write!(f, "Bool")
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

#[test]
fn test_parse1() {
    assert_eq!(Some(TypeId::Int32), TypeId::from_str("Int"));
    assert_eq!(Some(TypeId::Float32), TypeId::from_str("Float"));
    assert_eq!(Some(TypeId::Void), TypeId::from_str("Void"));
    assert_eq!(Some(TypeId::Bool), TypeId::from_str("Bool"));
}

#[test]
fn test_parse2() {
    assert_eq!(Some(TypeId::Array(Box::new(TypeId::Int32))), TypeId::from_str("Ref.Array[Int]"));
    assert_eq!(Some(TypeId::Array(Box::new(TypeId::Array(Box::new(TypeId::Int32))))), TypeId::from_str("Ref.Array[Ref.Array[Int]]"));
}

#[test]
fn test_parse3() {
    assert_eq!(Some(TypeId::Class("Point".to_owned())), TypeId::from_str("Ref.Point"));
    assert_eq!(Some(TypeId::Array(Box::new(TypeId::Class("Point".to_owned())))), TypeId::from_str("Ref.Array[Ref.Point]"));
    assert_eq!(Some(TypeId::Array(Box::new(TypeId::Array(Box::new(TypeId::Class("Point".to_owned())))))), TypeId::from_str("Ref.Array[Ref.Array[Ref.Point]]"));
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