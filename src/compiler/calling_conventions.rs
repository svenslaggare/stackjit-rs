use iced_x86::{Register};

use crate::compiler::{FunctionCompilationData, stack_layout};
use crate::model::function::{Function, FunctionSignature, FunctionDefinition};
use crate::model::typesystem::Type;
use crate::compiler::stack_layout::{STACK_ENTRY_SIZE, STACK_OFFSET};
use crate::ir::{InstructionIR, HardwareRegister};

pub struct CallingConventions {

}

impl CallingConventions {
    pub fn new() -> CallingConventions {
        CallingConventions {

        }
    }

    pub fn call_function_arguments(&self,
                                   function: &Function,
                                   compilation_data: &mut FunctionCompilationData,
                                   function_to_call: &FunctionSignature,
                                   instructions: &mut Vec<InstructionIR>) {
        for argument_index in (0..function_to_call.parameters.len()).rev() {
            self.call_function_argument(function, compilation_data, function_to_call, argument_index, instructions);
        }
    }

    pub fn call_function_argument(&self,
                                  function: &Function,
                                  compilation_data: &mut FunctionCompilationData,
                                  function_to_call: &FunctionSignature,
                                  argument_index: usize,
                                  instructions: &mut Vec<InstructionIR>) {
        let argument_type = &function_to_call.parameters[argument_index];
        match argument_type {
            Type::Float32 => {
                let relative_index = float_register_call_arguments::get_relative_index(&function_to_call.parameters, argument_index);
                if relative_index >= float_register_call_arguments::NUM_ARGUMENTS {
                    //Move from the operand stack to the normal stack
                    instructions.push(compilation_data.operand_stack.pop_register(
                        function,
                        HardwareRegister(Register::RAX)
                    ));

                    instructions.push(InstructionIR::PushNormalHardware(HardwareRegister(Register::RAX)));
                } else {
                    instructions.push(compilation_data.operand_stack.pop_register(
                        function,
                        HardwareRegister(float_register_call_arguments::get_argument(relative_index)))
                    );
                }
            }
            _ => {
                let relative_index = register_call_arguments::get_relative_index(&function_to_call.parameters, argument_index);
                if relative_index >= register_call_arguments::NUM_ARGUMENTS {
                    //Move from the operand stack to the normal stack
                    instructions.push(compilation_data.operand_stack.pop_register(
                        function,
                        HardwareRegister(Register::RAX)
                    ));

                    instructions.push(InstructionIR::PushNormalHardware(HardwareRegister(Register::RAX)));
                } else {
                    instructions.push(compilation_data.operand_stack.pop_register(
                        function,
                        HardwareRegister(register_call_arguments::get_argument(relative_index)))
                    );
                }
            }
        }
    }

    pub fn move_arguments_to_stack(&self, function: &Function, instructions: &mut Vec<InstructionIR>) {
        for argument_index in (0..function.definition().parameters().len()).rev() {
            match function.definition().parameters()[argument_index] {
                Type::Float32 => {
                    self.move_float_arguments_to_stack(
                        function,
                        argument_index,
                        float_register_call_arguments::get_relative_index(function.definition().parameters(), argument_index),
                        instructions
                    );
                }
                _ => {
                    self.move_non_float_arguments_to_stack(
                        function,
                        argument_index,
                        register_call_arguments::get_relative_index(function.definition().parameters(), argument_index),
                        instructions
                    );
                }
            }
        }
    }

    pub fn handle_return_value(&self, _function: &Function, instructions: &mut Vec<InstructionIR>, func_to_call: &FunctionDefinition) {
        match func_to_call.return_type() {
            Type::Void => {}
            Type::Float32 => {
                instructions.push(InstructionIR::PushOperandHardware(HardwareRegister(float_register_call_arguments::RETURN_VALUE)));
            }
            _ => {
                instructions.push(InstructionIR::PushOperandHardware(HardwareRegister(register_call_arguments::RETURN_VALUE)));
            }
        }
    }

    pub fn make_return_value(&self, function: &Function, instructions: &mut Vec<InstructionIR>) {
        match function.definition().return_type() {
            Type::Void => {}
            Type::Float32 => {
                instructions.push(InstructionIR::PopOperandHardware(HardwareRegister(float_register_call_arguments::RETURN_VALUE)));
            }
            _ => {
                instructions.push(InstructionIR::PopOperandHardware(HardwareRegister(register_call_arguments::RETURN_VALUE)));
            }
        }
    }

    fn move_non_float_arguments_to_stack(&self,
                                         function: &Function,
                                         argument_index: usize,
                                         relative_argument_index: usize,
                                         instructions: &mut Vec<InstructionIR>) {
        let argument_stack_offset = stack_layout::argument_stack_offset(function, argument_index as u32);
        if relative_argument_index >= register_call_arguments::NUM_ARGUMENTS {
            let stack_argument_index = self.get_stack_argument_index(function, argument_index);
            instructions.push(InstructionIR::MoveMemoryToHardware(
               HardwareRegister(Register::RAX),
               STACK_ENTRY_SIZE * (STACK_OFFSET as usize + stack_argument_index + 1) as i32
            ));

            instructions.push(InstructionIR::MoveHardwareToMemory(
                argument_stack_offset,
                HardwareRegister(Register::RAX)
            ));
        } else {
            instructions.push(InstructionIR::MoveHardwareToMemory(
                argument_stack_offset,
                HardwareRegister(register_call_arguments::get_argument(relative_argument_index))
            ));
        }
    }

    fn move_float_arguments_to_stack(&self,
                                     function: &Function,
                                     argument_index: usize,
                                     relative_argument_index: usize,
                                     instructions: &mut Vec<InstructionIR>) {
        let argument_stack_offset = stack_layout::argument_stack_offset(function, argument_index as u32);
        if relative_argument_index >= float_register_call_arguments::NUM_ARGUMENTS {
            let stack_argument_index = self.get_stack_argument_index(function, argument_index);
            instructions.push(InstructionIR::MoveMemoryToHardware(
                HardwareRegister(Register::RAX),
                STACK_ENTRY_SIZE * (STACK_OFFSET as usize + stack_argument_index + 1) as i32
            ));

            instructions.push(InstructionIR::MoveHardwareToMemory(
                argument_stack_offset,
                HardwareRegister(Register::RAX)
            ));
        } else {
            instructions.push(InstructionIR::MoveHardwareToMemory(
                argument_stack_offset,
                HardwareRegister(float_register_call_arguments::get_argument(relative_argument_index))
            ));
        }
    }

    fn get_stack_argument_index(&self,
                                function: &Function,
                                argument_index: usize) -> usize {
        let mut stack_argument_index = 0;

        let parameters = &function.definition().parameters();
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            match parameter {
                Type::Float32 => {
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

    pub fn num_stack_arguments(&self, parameters: &Vec<Type>) -> usize {
        let mut num_stack_arguments = 0;

        for (parameter_index, parameter) in parameters.iter().enumerate() {
            match parameter {
                Type::Float32 => {
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

    pub fn stack_alignment(&self, func_to_call: &FunctionDefinition) -> i32 {
        (self.num_stack_arguments(func_to_call.parameters()) % 2) as i32 * stack_layout::STACK_ENTRY_SIZE
    }
}

mod register_call_arguments {
    use iced_x86::Register;

    use crate::model::typesystem::Type;

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

    pub fn get_relative_index(parameters: &Vec<Type>, argument_index: usize) -> usize {
        let mut relative_argument_index = 0;
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            match parameter {
                Type::Float32 => {},
                _ => {
                    relative_argument_index += 1;
                }
            }
        }

        relative_argument_index
    }

    pub const RETURN_VALUE: Register = Register::RAX;
}

mod float_register_call_arguments {
    use iced_x86::Register;

    use crate::model::typesystem::Type;

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

    pub fn get_relative_index(parameters: &Vec<Type>, argument_index: usize) -> usize {
        let mut float_argument_index = 0;
        for (index, parameter) in parameters.iter().enumerate() {
            if index == argument_index {
                break;
            }

            if let Type::Float32 = parameter {
                float_argument_index += 1;
            }
        }

        float_argument_index
    }

    pub const RETURN_VALUE: Register = Register::XMM0;
}