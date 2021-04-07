use crate::model::typesystem::TypeId;
use crate::model::instruction::Instruction;

pub type FunctionAddress = *mut std::ffi::c_void;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionType {
    External,
    Managed
}

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    function_type: FunctionType,
    name: String,
    parameters: Vec<TypeId>,
    return_type: TypeId,
    address: Option<FunctionAddress>,
}

impl FunctionDeclaration {
    pub fn with_external(name: String, parameters: Vec<TypeId>, return_type: TypeId, address: FunctionAddress) -> FunctionDeclaration {
        FunctionDeclaration {
            function_type: FunctionType::External,
            name,
            parameters,
            return_type,
            address: Some(address)
        }
    }

    pub fn with_manager(name: String, parameters: Vec<TypeId>, return_type: TypeId) -> FunctionDeclaration {
        FunctionDeclaration {
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

    pub fn signature(&self) -> FunctionSignature {
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

impl std::fmt::Display for FunctionDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.signature(), self.return_type)
    }
}

pub struct Function {
    declaration: FunctionDeclaration,
    locals: Vec<TypeId>,
    instructions: Vec<Instruction>,
    instruction_operand_types: Vec<Vec<TypeId>>,
    operand_stack_size: usize
}

impl Function {
    pub fn new(declaration: FunctionDeclaration,
               locals: Vec<TypeId>,
               instructions: Vec<Instruction>) -> Function {
        let num_instructions = instructions.len();
        Function {
            declaration,
            locals,
            instructions,
            instruction_operand_types: (0..num_instructions).map(|_| Vec::new()).collect(),
            operand_stack_size: 0
        }
    }

    pub fn declaration(&self) -> &FunctionDeclaration {
        &self.declaration
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

pub struct FunctionStorage {
    functions: Vec<Box<Function>>
}

impl FunctionStorage {
    pub fn new() -> FunctionStorage {
        FunctionStorage {
            functions: Vec::new()
        }
    }

    pub fn add_function(&mut self, function: Function) {
        self.functions.push(Box::new(function));
    }

    pub fn get_function(&self, signature: &FunctionSignature) -> Option<&Function> {
        self.functions.iter()
            .find(|function| &function.declaration().signature() == signature)
            .map(|function| function.as_ref())
    }

    pub fn functions_mut(&mut self) -> &mut Vec<Box<Function>> {
        &mut self.functions
    }
}