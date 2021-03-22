use std::collections::HashMap;

use crate::model::typesystem::Type;

pub struct Field {
    name: String,
    field_type: Type
}

impl Field {
    pub fn new(name: String, field_type: Type) -> Field {
        Field {
            name,
            field_type
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn field_type(&self) -> &Type {
        &self.field_type
    }
}

pub struct Class {
    name: String,
    fields: Vec<Field>,
    field_offsets: HashMap<String, usize>,
    memory_size: usize
}

impl Class {
    pub fn new(name: String, fields: Vec<Field>) -> Class {
        let mut field_offsets = HashMap::new();
        let mut offset = 0;
        for field in &fields {
            field_offsets.insert(field.name.clone(), offset);
            offset += field.field_type.size();
        }

        Class {
            name,
            fields,
            field_offsets,
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
        self.fields.iter().find(|field| field.name == name)
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