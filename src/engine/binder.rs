use std::collections::HashMap;

use crate::model::function::{FunctionDefinition, FunctionAddress, FunctionSignature};

pub struct Binder {
    functions: HashMap<FunctionSignature, FunctionDefinition>
}

impl Binder {
    pub fn new() -> Binder {
        Binder {
            functions: HashMap::new()
        }
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