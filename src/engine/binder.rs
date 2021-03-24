use std::collections::HashMap;

use crate::model::function::{FunctionDefinition, FunctionAddress, FunctionSignature};
use crate::model::typesystem::Type;

pub struct Binder {
    functions: HashMap<FunctionSignature, FunctionDefinition>
}

impl Binder {
    pub fn new() -> Binder {
        let mut binder = Binder {
            functions: HashMap::new()
        };

        binder.define(FunctionDefinition::new_external(
            "std.gc.collect".to_owned(),
            Vec::new(),
            Type::Void,
            std::ptr::null_mut()
        ));

        binder
    }

    pub fn define(&mut self, definition: FunctionDefinition) {
        self.functions.insert(definition.call_signature(), definition);
    }

    pub fn get(&self, signature: &FunctionSignature) -> Option<&FunctionDefinition> {
        self.functions.get(signature)
    }

    pub fn set_address(&mut self, signature: &FunctionSignature, address: FunctionAddress) {
        self.functions.get_mut(signature).unwrap().set_address(address)
    }
}