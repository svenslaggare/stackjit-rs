use std::collections::HashMap;

use crate::model::typesystem::{TypeStorage};
use crate::model::function::{Function, FunctionSignature, FunctionDefinition};
use crate::ir::low::compiler::InstructionIRCompiler;
use crate::ir::low::InstructionIR;
use crate::ir::mid::compiler::InstructionMIRCompiler;
use crate::compiler::allocator::ExecutableMemoryAllocator;
use crate::compiler::code_generator::{CodeGenerator};
use crate::compiler::error_handling::ErrorHandling;
use crate::compiler::{FunctionCompilationData, FunctionCallType};
use crate::engine::binder::Binder;
use crate::ir::mid;

pub struct JitCompiler {
    memory_allocator: ExecutableMemoryAllocator,
    error_handling: ErrorHandling,
    compiled_functions: HashMap<FunctionSignature, FunctionCompilationData>,
}

impl JitCompiler {
    pub fn new() -> JitCompiler {
        let mut memory_allocator = ExecutableMemoryAllocator::new();
        let error_handling = ErrorHandling::new(&mut memory_allocator);

        JitCompiler {
            memory_allocator,
            error_handling,
            compiled_functions: HashMap::new(),
        }
    }

    pub fn compile_function(&mut self, binder: &mut Binder, type_storage: &mut TypeStorage, function: &Function) {
        println!("{}", function.definition().signature());
        println!("{{");

        let mut compilation_data = FunctionCompilationData::new();
        let instructions_ir = self.compile_ir(binder, function, &mut compilation_data);
        let function_code_bytes = self.generate_code(
            binder,
            type_storage,
            function,
            &mut compilation_data,
            &instructions_ir
        );

        println!("}}");
        println!();
        let function_code_ptr = self.memory_allocator.allocate(function_code_bytes.len());

        unsafe {
            function_code_ptr.copy_from(function_code_bytes.as_ptr() as *const _, function_code_bytes.len());
        }

        self.compiled_functions.insert(
            function.definition().call_signature(),
            compilation_data
        );

        binder.set_address(&function.definition().call_signature(), function_code_ptr);
    }

    pub fn resolve_calls_and_branches(&mut self, binder: &Binder) {
        for (signature, compiled_function) in &mut self.compiled_functions {
            if !compiled_function.unresolved_function_calls.is_empty() {
                JitCompiler::resolve_calls(binder, binder.get(signature).unwrap(), compiled_function);
            }

            if !compiled_function.unresolved_branches.is_empty() {
                JitCompiler::resolve_branches(binder.get(signature).unwrap(), compiled_function);
            }

            if !compiled_function.unresolved_native_branches.is_empty() {
                JitCompiler::resolve_native_branches(binder.get(signature).unwrap(), compiled_function);
            }
        }
    }

    fn resolve_calls(binder: &Binder,
                     function: &FunctionDefinition,
                     compiled_function: &mut FunctionCompilationData) {
        for unresolved_function_call in &compiled_function.unresolved_function_calls {
            let function_to_call = binder.get(&unresolved_function_call.signature).unwrap();

            match unresolved_function_call.call_type {
                FunctionCallType::Relative => {
                    let target = (function_to_call.address().unwrap() as i64
                        - (function.address().unwrap() as i64 + unresolved_function_call.call_offset as i64 + 5)) as i32;

                    unsafe {
                        let function_code_ptr = function.address().unwrap().add(unresolved_function_call.call_offset + 1) as *mut i32;
                        *function_code_ptr = target;
                    }
                }
                FunctionCallType::Absolute => {

                }
            }
        }

        compiled_function.unresolved_function_calls.clear();
    }

    fn resolve_branches(function: &FunctionDefinition, compiled_function: &mut FunctionCompilationData) {
        for (&branch_source, &(branch_target_label, branch_instruction_size)) in &compiled_function.unresolved_branches {
            let branch_target = compiled_function.branch_targets[&branch_target_label];

            let target = branch_target as i32 - branch_source as i32 - branch_instruction_size as i32;
            let source_offset = branch_source as i32 + branch_instruction_size as i32 - std::mem::size_of::<i32>() as i32;

            unsafe {
                let code_ptr = function.address().unwrap().add(source_offset as usize) as *mut i32;
                *code_ptr = target;
            }
        }

        compiled_function.unresolved_branches.clear();
    }

    fn resolve_native_branches(function: &FunctionDefinition, compiled_function: &mut FunctionCompilationData) {
        let function_code_ptr = function.address().unwrap();
        for (&source, &target) in &compiled_function.unresolved_native_branches {
            let native_target = (target as isize - (function_code_ptr as u64 + source as u64) as isize - 6) as i32;
            let source_offset = source + 6 - std::mem::size_of::<i32>();

            unsafe {
                let code_ptr = function_code_ptr.add(source_offset as usize) as *mut i32;
                *code_ptr = native_target;
            }
        }

        compiled_function.unresolved_native_branches.clear();
    }

    fn compile_ir(&self,
                  binder: &Binder,
                  function: &Function,
                  compilation_data: &mut FunctionCompilationData) -> Vec<InstructionIR> {
        // let mut ir_compiler = InstructionIRCompiler::new(&binder, function, compilation_data);
        // ir_compiler.compile(function.instructions());
        // ir_compiler.done()

        let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function, compilation_data);
        mir_compiler.compile(function.instructions());
        let mir_result = mir_compiler.done();

        let mut ir_compiler = mid::ir_compiler::InstructionIRCompiler::new(&binder, &function, compilation_data);
        ir_compiler.compile(&mir_result);
        ir_compiler.done()
    }

    fn generate_code(&self,
                     binder: &Binder,
                     type_storage: &mut TypeStorage,
                     function: &Function,
                     compilation_data: &mut FunctionCompilationData,
                     instructions_ir: &Vec<InstructionIR>) -> Vec<u8> {
        let mut code_generator = CodeGenerator::new(binder, &self.error_handling, type_storage);
        code_generator.generate(function, compilation_data, instructions_ir);
        code_generator.done()
    }
}
