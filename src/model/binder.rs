use std::collections::HashMap;

use crate::model::function::{FunctionDeclaration, FunctionAddress, FunctionSignature};
use crate::model::typesystem::TypeId;

pub struct Binder {
    functions: HashMap<FunctionSignature, FunctionDeclaration>
}

impl Binder {
    pub fn new() -> Binder {
        let mut binder = Binder {
            functions: HashMap::new()
        };

        binder.define(FunctionDeclaration::new_external(
            "std.gc.collect".to_owned(),
            Vec::new(),
            TypeId::Void,
            std::ptr::null_mut()
        ));

        binder.define(FunctionDeclaration::new_external(
            "std.gc.print_stack_frame".to_owned(),
            Vec::new(),
            TypeId::Void,
            std::ptr::null_mut()
        ));

        binder
    }

    pub fn define(&mut self, definition: FunctionDeclaration) {
        self.functions.insert(definition.signature(), definition);
    }

    pub fn get(&self, signature: &FunctionSignature) -> Option<&FunctionDeclaration> {
        self.functions.get(signature)
    }

    pub fn set_address(&mut self, signature: &FunctionSignature, address: FunctionAddress) {
        self.functions.get_mut(signature).unwrap().set_address(address)
    }
}