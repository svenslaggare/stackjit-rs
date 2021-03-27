use std::collections::HashMap;

use crate::analysis::{AnalysisResult, null_check_elision};
use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::compiler::{FunctionCallType, FunctionCompilationData};
use crate::compiler::ir::allocated_compiler::AllocatedInstructionIRCompiler;
use crate::compiler::allocator::ExecutableMemoryAllocator;
use crate::compiler::code_generator::{CodeGenerator, CodeGeneratorResult};
use crate::compiler::error_handling::ErrorHandling;
use crate::compiler::ir::InstructionIR;
use crate::compiler::ir::compiler::InstructionIRCompiler;
use crate::engine::binder::Binder;
use crate::mir;
use crate::mir::branches;
use crate::mir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::model::function::{Function, FunctionDefinition, FunctionSignature};
use crate::model::typesystem::TypeStorage;

pub struct JitCompiler {
    memory_allocator: ExecutableMemoryAllocator,
    error_handling: ErrorHandling,
    functions_compilation_data: HashMap<FunctionSignature, FunctionCompilationData>,
}

impl JitCompiler {
    pub fn new() -> JitCompiler {
        let mut memory_allocator = ExecutableMemoryAllocator::new();
        let error_handling = ErrorHandling::new(&mut memory_allocator);

        JitCompiler {
            memory_allocator,
            error_handling,
            functions_compilation_data: HashMap::new(),
        }
    }

    pub fn compile_function(&mut self,
                            binder: &mut Binder,
                            type_storage: &mut TypeStorage,
                            function: &Function) {
        println!("{}", function.definition().signature());
        println!("{{");

        let (compilation_result, instructions_ir) = self.compile_ir(binder, type_storage, function);
        let mut compilation_data = FunctionCompilationData::new(compilation_result);
        let generator_result = self.generate_code(
            binder,
            type_storage,
            function,
            &mut compilation_data,
            &instructions_ir
        );
        compilation_data.instructions_offsets = generator_result.instructions_offsets;

        println!("}}");
        println!();
        let function_code_ptr = self.memory_allocator.allocate(generator_result.code_bytes.len());

        unsafe {
            function_code_ptr.copy_from(
                generator_result.code_bytes.as_ptr() as *const _,
                generator_result.code_bytes.len()
            );
        }

        self.functions_compilation_data.insert(function.definition().call_signature(), compilation_data);

        binder.set_address(&function.definition().call_signature(), function_code_ptr);
    }

    pub fn get_compilation_data(&self, signature: &FunctionSignature) -> Option<&FunctionCompilationData> {
        self.functions_compilation_data.get(signature)
    }

    pub fn resolve_calls_and_branches(&mut self, binder: &Binder) {
        for (signature, compilation_data) in &mut self.functions_compilation_data {
            if !compilation_data.unresolved_function_calls.is_empty() {
                JitCompiler::resolve_calls(binder, binder.get(signature).unwrap(), compilation_data);
            }

            if !compilation_data.unresolved_branches.is_empty() {
                JitCompiler::resolve_branches(binder.get(signature).unwrap(), compilation_data);
            }

            if !compilation_data.unresolved_native_branches.is_empty() {
                JitCompiler::resolve_native_branches(binder.get(signature).unwrap(), compilation_data);
            }
        }
    }

    fn resolve_calls(binder: &Binder,
                     function: &FunctionDefinition,
                     compilation_data: &mut FunctionCompilationData) {
        for unresolved_function_call in &compilation_data.unresolved_function_calls {
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
                    unimplemented!();
                }
            }
        }

        compilation_data.unresolved_function_calls.clear();
    }

    fn resolve_branches(function: &FunctionDefinition, compilation_data: &mut FunctionCompilationData) {
        for (&branch_source, &(branch_target_label, branch_instruction_size)) in &compilation_data.unresolved_branches {
            let branch_target = compilation_data.branch_targets[&branch_target_label];

            let target = branch_target as i32 - branch_source as i32 - branch_instruction_size as i32;
            let source_offset = branch_source as i32 + branch_instruction_size as i32 - std::mem::size_of::<i32>() as i32;

            unsafe {
                let code_ptr = function.address().unwrap().add(source_offset as usize) as *mut i32;
                *code_ptr = target;
            }
        }

        compilation_data.unresolved_branches.clear();
    }

    fn resolve_native_branches(function: &FunctionDefinition, compilation_data: &mut FunctionCompilationData) {
        let function_code_ptr = function.address().unwrap();
        for (&source, &target) in &compilation_data.unresolved_native_branches {
            let native_target = (target as isize - (function_code_ptr as u64 + source as u64) as isize - 6) as i32;
            let source_offset = source + 6 - std::mem::size_of::<i32>();

            unsafe {
                let code_ptr = function_code_ptr.add(source_offset as usize) as *mut i32;
                *code_ptr = native_target;
            }
        }

        compilation_data.unresolved_native_branches.clear();
    }

    fn compile_ir(&self,
                  binder: &Binder,
                  type_storage: &TypeStorage,
                  function: &Function) -> (MIRCompilationResult, Vec<InstructionIR>) {
        let mut mir_compiler = InstructionMIRCompiler::new(&binder, &function);
        mir_compiler.compile(function.instructions());
        let compilation_result = mir_compiler.done();

        let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
        let control_flow_graph = ControlFlowGraph::new(
            &compilation_result.instructions,
            &basic_blocks,
        );

        let analysis_result = AnalysisResult {
            instructions_register_null_status: null_check_elision::compute_null_check_elision(
                function,
                &compilation_result,
                &basic_blocks,
                &control_flow_graph
            )
        };

        // let mut ir_compiler = InstructionIRCompiler::new(&binder, &type_storage, &function, &compilation_result, &analysis_result);
        let mut ir_compiler = AllocatedInstructionIRCompiler::new(&binder, &type_storage, &function, &compilation_result, &analysis_result);
        ir_compiler.compile();
        let instructions_ir = ir_compiler.done();
        (compilation_result, instructions_ir)
    }

    fn generate_code(&self,
                     binder: &Binder,
                     type_storage: &mut TypeStorage,
                     function: &Function,
                     compilation_data: &mut FunctionCompilationData,
                     instructions_ir: &Vec<InstructionIR>) -> CodeGeneratorResult {
        let mut code_generator = CodeGenerator::new(binder, &self.error_handling, type_storage);
        code_generator.generate(function, compilation_data, instructions_ir);
        code_generator.done()
    }
}
