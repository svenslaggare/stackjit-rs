use iced_x86::Register;

use crate::compiler::ir::{HardwareRegister, HardwareRegisterExplicit, InstructionIR, Variable};
use crate::compiler::stack_layout;
use crate::compiler::stack_layout::{STACK_ENTRY_SIZE, STACK_OFFSET};
use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::typesystem::TypeId;

pub struct CallingConventions {

}

impl CallingConventions {
    pub fn new() -> CallingConventions {
        CallingConventions {

        }
    }

    pub fn call_function_arguments(&self,
                                   function_to_call: &FunctionDeclaration,
                                   arguments: &Vec<Variable>,
                                   instructions: &mut Vec<InstructionIR>) {
        for argument_index in (0..function_to_call.parameters().len()).rev() {
            self.call_function_argument(
                function_to_call,
                arguments,
                argument_index,
                instructions
            );
        }
    }

    pub fn call_function_argument(&self,
                                  function_to_call: &FunctionDeclaration,
                                  arguments: &Vec<Variable>,
                                  argument_index: usize,
                                  instructions: &mut Vec<InstructionIR>) {
        let argument_source = &arguments[argument_index];

        match &function_to_call.parameters()[argument_index] {
            TypeId::Float32 => {
                let relative_index = float_register_call_arguments::get_relative_index(&function_to_call.parameters(), argument_index);
                if relative_index >= float_register_call_arguments::NUM_ARGUMENTS {
                    argument_source.move_to_stack(instructions);
                } else {
                    argument_source.move_to_explicit(
                        HardwareRegisterExplicit(float_register_call_arguments::get_argument(relative_index)),
                        instructions
                    );
                }
            }
            _ => {
                let relative_index = register_call_arguments::get_relative_index(&function_to_call.parameters(), argument_index);
                if relative_index >= register_call_arguments::NUM_ARGUMENTS {
                    argument_source.move_to_stack(instructions);
                } else {
                    argument_source.move_to_explicit(
                        HardwareRegisterExplicit(register_call_arguments::get_argument(relative_index)),
                        instructions
                    );
                }
            }
        }
    }

    pub fn move_arguments_to_stack(&self, function: &Function, instructions: &mut Vec<InstructionIR>) -> Vec<Variable> {
        let mut argument_sources = Vec::new();

        for argument_index in (0..function.declaration().parameters().len()).rev() {
            let argument_source = match function.declaration().parameters()[argument_index] {
                TypeId::Float32 => {
                    self.move_float_arguments_to_stack(
                        function,
                        argument_index,
                        float_register_call_arguments::get_relative_index(function.declaration().parameters(), argument_index)
                    )
                }
                _ => {
                    self.move_non_float_arguments_to_stack(
                        function,
                        argument_index,
                        register_call_arguments::get_relative_index(function.declaration().parameters(), argument_index)
                    )
                }
            };

            argument_source.move_to_stack_frame(
                stack_layout::argument_stack_offset(function, argument_index as u32),
                instructions
            );

            argument_sources.push(argument_source);
        }

        argument_sources.reverse();
        argument_sources
    }

    pub fn handle_return_value(&self,
                               _function: &Function,
                               variable: &Variable,
                               func_to_call: &FunctionDeclaration,
                               instructions: &mut Vec<InstructionIR>) {
        match func_to_call.return_type() {
            TypeId::Void => {}
            TypeId::Float32 => {
                variable.move_from_explicit(
                    HardwareRegisterExplicit(float_register_call_arguments::RETURN_VALUE),
                    instructions
                );
            }
            _ => {
                variable.move_from_explicit(
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE),
                    instructions
                );
            }
        }
    }

    pub fn make_return_value(&self,
                             function: &Function,
                             variable: &Variable,
                             instructions: &mut Vec<InstructionIR>) {
        match function.declaration().return_type() {
            TypeId::Void => {}
            TypeId::Float32 => {
                variable.move_to_explicit(
                    HardwareRegisterExplicit(float_register_call_arguments::RETURN_VALUE),
                    instructions
                );
            }
            _ => {
                variable.move_to_explicit(
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE),
                    instructions
                );
            }
        }
    }

    fn move_non_float_arguments_to_stack(&self,
                                         function: &Function,
                                         argument_index: usize,
                                         relative_argument_index: usize) -> Variable {
        if relative_argument_index >= register_call_arguments::NUM_ARGUMENTS {
            let stack_argument_index = self.get_stack_argument_index(function, argument_index);
            let argument_source_offset = STACK_ENTRY_SIZE * (STACK_OFFSET as usize + stack_argument_index + 1) as i32;
            Variable::FrameMemory(argument_source_offset)
        } else {
            let argument_source_register = HardwareRegisterExplicit(register_call_arguments::get_argument(relative_argument_index));
            Variable::RegisterExplicit(argument_source_register)
        }
    }

    fn move_float_arguments_to_stack(&self,
                                     function: &Function,
                                     argument_index: usize,
                                     relative_argument_index: usize) -> Variable {
        if relative_argument_index >= float_register_call_arguments::NUM_ARGUMENTS {
            let stack_argument_index = self.get_stack_argument_index(function, argument_index);
            let argument_source_offset = STACK_ENTRY_SIZE * (STACK_OFFSET as usize + stack_argument_index + 1) as i32;
            Variable::FrameMemory(argument_source_offset)
        } else {
            let argument_source_register = HardwareRegisterExplicit(float_register_call_arguments::get_argument(relative_argument_index));
            Variable::RegisterExplicit(argument_source_register)
        }
    }

    fn get_stack_argument_index(&self,
                                function: &Function,
                                argument_index: usize) -> usize {
        let mut stack_argument_index = 0;

        let parameters = &function.declaration().parameters();
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            match parameter {
                TypeId::Float32 => {
                    if float_register_call_arguments::get_relative_index(parameters, index) >= float_register_call_arguments::NUM_ARGUMENTS {
                        stack_argument_index += 1;
                    }
                }
                _ => {
                    if register_call_arguments::get_relative_index(parameters, index) >= register_call_arguments::NUM_ARGUMENTS {
                        stack_argument_index += 1;
                    }
                }
            }
        }

        stack_argument_index
    }

    pub fn num_stack_arguments(&self, parameters: &Vec<TypeId>) -> usize {
        let mut num_stack_arguments = 0;

        for (parameter_index, parameter) in parameters.iter().enumerate() {
            match parameter {
                TypeId::Float32 => {
                    if float_register_call_arguments::get_relative_index(parameters, parameter_index) >= float_register_call_arguments::NUM_ARGUMENTS {
                        num_stack_arguments += 1;
                    }
                }
                _ => {
                    if register_call_arguments::get_relative_index(parameters, parameter_index) >= register_call_arguments::NUM_ARGUMENTS {
                        num_stack_arguments += 1;
                    }
                }
            }
        }

        num_stack_arguments
    }

    pub fn stack_alignment(&self, func_to_call: &FunctionDeclaration, num_saved: usize) -> i32 {
        ((self.num_stack_arguments(func_to_call.parameters()) + num_saved) % 2) as i32 * stack_layout::STACK_ENTRY_SIZE
    }
}

pub fn get_call_register(func_to_call: &FunctionDeclaration, index: usize, argument_type: &TypeId) -> Option<Register> {
    match argument_type {
        TypeId::Float32 => {
            let relative_index = float_register_call_arguments::get_relative_index(func_to_call.parameters(), index);
            if relative_index < float_register_call_arguments::NUM_ARGUMENTS {
                return Some(float_register_call_arguments::get_argument(relative_index));
            }
        }
        _ => {
            let relative_index = register_call_arguments::get_relative_index(func_to_call.parameters(), index);
            if relative_index < register_call_arguments::NUM_ARGUMENTS {
                return Some(register_call_arguments::get_argument(relative_index));
            }
        }
    }

    None
}

pub mod register_call_arguments {
    use iced_x86::Register;

    use crate::model::typesystem::TypeId;

    pub const ARG0: Register = Register::RDI;
    pub const ARG1: Register = Register::RSI;
    pub const ARG2: Register = Register::RDX;
    pub const ARG3: Register = Register::RCX;
    pub const ARG4: Register = Register::R8;
    pub const ARG5: Register = Register::R9;
    pub const NUM_ARGUMENTS: usize = 6;

    pub fn get_argument(index: usize) -> Register {
        match index {
            0 => ARG0,
            1 => ARG1,
            2 => ARG2,
            3 => ARG3,
            4 => ARG4,
            5 => ARG5,
            _ => panic!("invalid index.")
        }
    }

    pub fn get_relative_index(parameters: &Vec<TypeId>, argument_index: usize) -> usize {
        let mut relative_argument_index = 0;
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            match parameter {
                TypeId::Float32 => {},
                _ => {
                    relative_argument_index += 1;
                }
            }
        }

        relative_argument_index
    }

    pub const RETURN_VALUE: Register = Register::RAX;
    pub const RETURN_VALUE_32: Register = Register::EAX;
}

pub mod float_register_call_arguments {
    use iced_x86::Register;

    use crate::model::typesystem::TypeId;

    pub const ARG0: Register = Register::XMM0;
    pub const ARG1: Register = Register::XMM1;
    pub const ARG2: Register = Register::XMM2;
    pub const ARG3: Register = Register::XMM3;
    pub const ARG4: Register = Register::XMM4;
    pub const ARG5: Register = Register::XMM5;
    pub const ARG6: Register = Register::XMM6;
    pub const ARG7: Register = Register::XMM7;
    pub const NUM_ARGUMENTS: usize = 8;

    pub fn get_argument(index: usize) -> Register {
        match index {
            0 => ARG0,
            1 => ARG1,
            2 => ARG2,
            3 => ARG3,
            4 => ARG4,
            5 => ARG5,
            6 => ARG6,
            7 => ARG7,
            _ => panic!("invalid index.")
        }
    }

    pub fn get_relative_index(parameters: &Vec<TypeId>, argument_index: usize) -> usize {
        let mut float_argument_index = 0;
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            if let TypeId::Float32 = parameter {
                float_argument_index += 1;
            }
        }

        float_argument_index
    }

    pub const RETURN_VALUE: Register = Register::XMM0;
}