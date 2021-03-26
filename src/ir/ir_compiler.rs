use crate::compiler::calling_conventions::{CallingConventions, register_call_arguments, float_register_call_arguments};
use crate::compiler::stack_layout;
use crate::engine::binder::Binder;
use crate::ir::{HardwareRegister, HardwareRegisterExplicit, InstructionIR, Variable};
use crate::ir::mid::{InstructionMIR, RegisterMIR};
use crate::ir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::ir::mid::InstructionMIRData;
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{Type, TypeStorage};
use crate::model::verifier::Verifier;
use crate::analysis::AnalysisResult;

pub struct InstructionIRCompiler<'a> {
    binder: &'a Binder,
    type_storage: &'a TypeStorage,
    function: &'a Function,
    compilation_result: &'a MIRCompilationResult,
    analysis_result: &'a AnalysisResult,
    instructions: Vec<InstructionIR>
}

impl<'a> InstructionIRCompiler<'a> {
    pub fn new(binder: &'a Binder,
               type_storage: &'a TypeStorage,
               function: &'a Function,
               compilation_result: &'a MIRCompilationResult,
               analysis_result: &'a AnalysisResult) -> InstructionIRCompiler<'a> {
        InstructionIRCompiler {
            binder,
            type_storage,
            function,
            compilation_result,
            analysis_result,
            instructions: Vec::new()
        }
    }

    pub fn compile(&mut self) {
        self.compile_initialize_function();

        for (instruction_index, instruction) in self.compilation_result.instructions.iter().enumerate() {
            self.compile_instruction(instruction_index, instruction);
        }
    }

    fn compile_initialize_function(&mut self) {
        self.instructions.push(InstructionIR::InitializeFunction);

        let stack_size = stack_layout::stack_size(self.function, self.compilation_result);
        if stack_size > 0 {
            self.instructions.push(InstructionIR::SubFromStackPointer(stack_size));
        }

        CallingConventions::new().move_arguments_to_stack(self.function, &mut self.instructions);

        if !self.compilation_result.need_zero_initialize_registers.is_empty() {
            self.instructions.push(InstructionIR::LoadZeroToRegister(HardwareRegister::Int(0)));
            for register in &self.compilation_result.need_zero_initialize_registers {
                self.instructions.push(InstructionIR::StoreFrameMemory(
                    self.get_register_stack_offset(register),
                    HardwareRegister::Int(0)
                ));
            }
        }
    }

    fn compile_instruction(&mut self, instruction_index: usize, instruction: &InstructionMIR) {
        self.instructions.push(InstructionIR::Marker(instruction.index, instruction_index));

        match &instruction.data {
            InstructionMIRData::LoadInt32(destination, value) => {
                self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), *value));
            }
            InstructionMIRData::LoadFloat32(destination, value) => {
                let value: i32 = unsafe { std::mem::transmute(*value) };
                self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), value));
            }
            InstructionMIRData::Move(destination, source) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(source)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::AddInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::SubInt32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubInt32(HardwareRegister::Int(0), HardwareRegister::Int(1)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::Int(0)));
            }
            InstructionMIRData::AddFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::AddFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIRData::SubFloat32(destination, operand1, operand2) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
                self.instructions.push(InstructionIR::SubFloat32(HardwareRegister::Float(0), HardwareRegister::Float(1)));
                self.instructions.push(InstructionIR::StoreFrameMemory(self.get_register_stack_offset(destination), HardwareRegister::Float(0)));
            }
            InstructionMIRData::Return(source) => {
                if let Some(source) = source {
                    CallingConventions::new().make_return_value(
                        self.function,
                        &Variable::FrameMemory(self.get_register_stack_offset(source)),
                        &mut self.instructions
                    );
                }

                self.instructions.push(InstructionIR::Return);
            }
            InstructionMIRData::Call(signature, return_value, arguments) => {
                let func_to_call = self.binder.get(signature).unwrap();

                let arguments_source = arguments
                    .iter()
                    .map(|argument| Variable::FrameMemory(self.get_register_stack_offset(argument)))
                    .collect::<Vec<_>>();

                self.instructions.push(InstructionIR::Call(signature.clone(), arguments_source, 0));

                if let Some(return_value) = return_value {
                    CallingConventions::new().handle_return_value(
                        self.function,
                        &Variable::FrameMemory(self.get_register_stack_offset(return_value)),
                        func_to_call,
                        &mut self.instructions
                    );
                }
            }
            InstructionMIRData::LoadArgument(argument_index, destination) => {
                let argument_offset = stack_layout::argument_stack_offset(self.function, *argument_index);
                let register_offset = self.get_register_stack_offset(destination);

                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), argument_offset));
                self.instructions.push(InstructionIR::StoreFrameMemory(register_offset, HardwareRegister::Int(0)));
            }
            InstructionMIRData::LoadNull(destination) => {
                self.instructions.push(InstructionIR::MoveInt32ToFrameMemory(self.get_register_stack_offset(destination), 0));
            }
            InstructionMIRData::NewArray(element, destination, size) => {
                self.instructions.push(InstructionIR::PrintStackFrame(instruction_index));

                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(size)));
                self.instructions.push(InstructionIR::NewArray(element.clone(), HardwareRegister::Int(0), 0));
                self.instructions.push(InstructionIR::StoreFrameMemoryExplicit(
                    self.get_register_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIRData::LoadElement(element, destination, array_ref, index) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(1), self.get_register_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                let return_value = match element {
                    Type::Float32 => HardwareRegister::Float(2),
                    _ => HardwareRegister::Int(2)
                };

                self.instructions.push(InstructionIR::LoadElement(element.clone(), return_value, HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::StoreFrameMemory(
                    self.get_register_stack_offset(destination),
                    return_value
                ));
            }
            InstructionMIRData::StoreElement(element, array_ref, index, value) => {
                let value_register = match element {
                    Type::Float32 => HardwareRegister::Float(2),
                    _ => HardwareRegister::Int(2)
                };

                self.instructions.push(InstructionIR::LoadFrameMemory(value_register, self.get_register_stack_offset(value)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(1), self.get_register_stack_offset(index)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                self.instructions.push(InstructionIR::ArrayBoundsCheck(HardwareRegister::Int(0), HardwareRegister::Int(1)));

                self.instructions.push(InstructionIR::StoreElement(
                    element.clone(),
                    HardwareRegister::Int(0),
                    HardwareRegister::Int(1),
                    value_register
                ));
            }
            InstructionMIRData::LoadArrayLength(destination, array_ref) => {
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(array_ref)));

                if self.can_be_null(instruction_index, array_ref) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                self.instructions.push(InstructionIR::LoadArrayLength(HardwareRegister::IntSpill, HardwareRegister::Int(0)));
                self.instructions.push(InstructionIR::StoreFrameMemory(
                    self.get_register_stack_offset(destination),
                    HardwareRegister::IntSpill
                ));
            }
            InstructionMIRData::NewObject(class_type, destination) => {
                self.instructions.push(InstructionIR::PrintStackFrame(instruction_index));

                self.instructions.push(InstructionIR::NewObject(class_type.clone()));
                self.instructions.push(InstructionIR::StoreFrameMemoryExplicit(
                    self.get_register_stack_offset(destination),
                    HardwareRegisterExplicit(register_call_arguments::RETURN_VALUE)
                ));
            }
            InstructionMIRData::LoadField(class_type, field_name, destination, class_reference) => {
                let class = self.type_storage.get(class_type).unwrap().class.as_ref().unwrap();
                let field = class.get_field(field_name).unwrap();

                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(class_reference)));

                if self.can_be_null(instruction_index, class_reference) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                let return_value = match field.field_type() {
                    Type::Float32 => HardwareRegister::Float(1),
                    _ => HardwareRegister::Int(1)
                };

                self.instructions.push(InstructionIR::LoadField(
                    field.field_type().clone(),
                    field.offset(),
                    return_value,
                    HardwareRegister::Int(0)
                ));

                self.instructions.push(InstructionIR::StoreFrameMemory(
                    self.get_register_stack_offset(destination),
                    return_value
                ));
            }
            InstructionMIRData::StoreField(class_type, field_name, class_reference, value) => {
                let class = self.type_storage.get(class_type).unwrap().class.as_ref().unwrap();
                let field = class.get_field(field_name).unwrap();

                let value_register = match field.field_type() {
                    Type::Float32 => HardwareRegister::Float(1),
                    _ => HardwareRegister::Int(1)
                };

                self.instructions.push(InstructionIR::LoadFrameMemory(value_register, self.get_register_stack_offset(value)));
                self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(class_reference)));

                if self.can_be_null(instruction_index, class_reference) {
                    self.instructions.push(InstructionIR::NullReferenceCheck(HardwareRegister::Int(0)));
                }

                self.instructions.push(InstructionIR::StoreField(
                    field.field_type().clone(),
                    field.offset(),
                    HardwareRegister::Int(0),
                    value_register,
                ));
            }
            InstructionMIRData::GarbageCollect => {
                self.instructions.push(InstructionIR::GarbageCollect(instruction_index));
            }
            InstructionMIRData::BranchLabel(label) => {
                self.instructions.push(InstructionIR::BranchLabel(*label));
            }
            InstructionMIRData::Branch(label) => {
                self.instructions.push(InstructionIR::Branch(*label));
            }
            InstructionMIRData::BranchCondition(condition, compare_type, label, operand1, operand2) => {
                let signed = match compare_type {
                    Type::Int32 => {
                        self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(0), self.get_register_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Int(1), self.get_register_stack_offset(operand2)));
                        self.instructions.push(InstructionIR::Compare(Type::Int32, HardwareRegister::Int(0), HardwareRegister::Int(1)));
                        true
                    }
                    Type::Float32 => {
                        self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(0), self.get_register_stack_offset(operand1)));
                        self.instructions.push(InstructionIR::LoadFrameMemory(HardwareRegister::Float(1), self.get_register_stack_offset(operand2)));
                        self.instructions.push(InstructionIR::Compare(Type::Float32, HardwareRegister::Float(0), HardwareRegister::Float(1)));
                        false
                    }
                    _ => { panic!("unexpected."); }
                };

                self.instructions.push(InstructionIR::BranchCondition(
                    *condition,
                    signed,
                    *label
                ));
            }
        }
    }

    fn can_be_null(&self, instruction_index: usize, register: &RegisterMIR) -> bool {
        assert!(register.value_type.is_reference());
        self.analysis_result.instructions_register_null_status[instruction_index].get(register).cloned().unwrap_or(true)
    }

    fn get_register_stack_offset(&self, register: &RegisterMIR) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function, register.number)
    }

    pub fn done(self) -> Vec<InstructionIR> {
        self.instructions
    }
}
