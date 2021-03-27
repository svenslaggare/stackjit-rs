use crate::model::typesystem::TypeId;
use crate::model::instruction::Instruction;

pub type FunctionAddress = *mut std::ffi::c_void;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionType {
    External,
    Managed
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    function_type: FunctionType,
    name: String,
    parameters: Vec<TypeId>,
    return_type: TypeId,
    address: Option<FunctionAddress>,
}

impl FunctionDefinition {
    pub fn new_external(name: String, parameters: Vec<TypeId>, return_type: TypeId, address: FunctionAddress) -> FunctionDefinition {
        FunctionDefinition {
            function_type: FunctionType::External,
            name,
            parameters,
            return_type,
            address: Some(address)
        }
    }

    pub fn new_managed(name: String, parameters: Vec<TypeId>, return_type: TypeId) -> FunctionDefinition {
        FunctionDefinition {
            function_type: FunctionType::Managed,
            name,
            parameters,
            return_type,
            address: None
        }
    }

    pub fn function_type(&self) -> &FunctionType {
        &self.function_type
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parameters(&self) -> &Vec<TypeId> {
        &self.parameters
    }

    pub fn return_type(&self) -> &TypeId {
        &self.return_type
    }

    pub fn address(&self) -> Option<FunctionAddress> {
        self.address
    }

    pub fn signature(&self) -> String {
        format!("{} {}", self.call_signature(), self.return_type)
    }

    pub fn call_signature(&self) -> FunctionSignature {
        FunctionSignature::new(self.name.clone(), self.parameters.clone())
    }

    pub fn set_address(&mut self, address: FunctionAddress) {
        self.address = Some(address);
    }

    pub fn is_entry_point(&self) -> bool {
        &self.function_type == &FunctionType::Managed
        && self.name() == "main"
        && self.parameters().is_empty()
        && self.return_type() == &TypeId::Int32
    }
}

pub struct Function {
    definition: FunctionDefinition,
    locals: Vec<TypeId>,
    instructions: Vec<Instruction>,
    instruction_operand_types: Vec<Vec<TypeId>>,
    operand_stack_size: usize
}

impl Function {
    pub fn new(definition: FunctionDefinition,
               locals: Vec<TypeId>,
               instructions: Vec<Instruction>) -> Function {
        let num_instructions = instructions.len();
        Function {
            definition,
            locals,
            instructions,
            instruction_operand_types: (0..num_instructions).map(|_| Vec::new()).collect(),
            operand_stack_size: 0
        }
    }

    pub fn definition(&self) -> &FunctionDefinition {
        &self.definition
    }

    pub fn locals(&self) -> &Vec<TypeId> {
        &self.locals
    }

    pub fn operand_stack_size(&self) -> usize {
        self.operand_stack_size
    }

    pub fn set_operand_stack_size(&mut self, value: usize) {
        self.operand_stack_size = value;
    }

    pub fn instructions(&self) -> &Vec<Instruction> {
        &self.instructions
    }

    pub fn instruction_operand_types(&self, index: usize) -> &Vec<TypeId> {
        &self.instruction_operand_types[index]
    }

    pub fn instruction_operand_types_mut(&mut self, index: usize) -> &mut Vec<TypeId> {
        &mut self.instruction_operand_types[index]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<TypeId>
}

impl FunctionSignature {
    pub fn new(name: String, parameters: Vec<TypeId>) -> FunctionSignature {
        FunctionSignature {
            name,
            parameters
        }
    }
}

impl std::fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.parameters.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" "))
    }
}