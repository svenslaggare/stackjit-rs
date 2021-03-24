use std::collections::HashMap;

use crate::model::typesystem::Type;
use std::iter::FromIterator;

pub struct Field {
    name: String,
    field_type: Type,
    offset: usize
}

impl Field {
    pub fn new(name: String, field_type: Type) -> Field {
        Field {
            name,
            field_type,
            offset: 0
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn field_type(&self) -> &Type {
        &self.field_type
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

pub struct Class {
    name: String,
    fields: Vec<Field>,
    fields_mapping: HashMap<String, usize>,
    memory_size: usize
}

impl Class {
    pub fn new(name: String, mut fields: Vec<Field>) -> Class {
        let mut offset = 0;
        for field in &mut fields {
            field.offset = offset;
            offset += field.field_type.size();
        }

        let fields_mapping = HashMap::from_iter(
            fields.iter().enumerate().map(|(index, field)| (field.name.clone(), index))
        );

        Class {
            name,
            fields,
            fields_mapping,
            memory_size: offset
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &Vec<Field> {
        &self.fields
    }

    pub fn get_field(&self, name: &str) -> Option<&Field> {
        let field_index = self.fields_mapping.get(name)?;
        self.fields.get(*field_index)
    }

    pub fn memory_size(&self) -> usize {
        self.memory_size
    }
}

pub struct ClassProvider {
    classes: HashMap<String, Class>
}

impl ClassProvider {
    pub fn new() -> ClassProvider {
        ClassProvider {
            classes: HashMap::new()
        }
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Option<&Class> {
        self.classes.get(name)
    }

    pub fn define(&mut self, class: Class) {
        let class_name = class.name.clone();
        self.classes.insert(class_name, class);
    }
}