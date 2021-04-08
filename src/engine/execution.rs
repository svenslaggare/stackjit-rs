use crate::compiler::jit::{JitCompiler, JitSettings};
use crate::model::function::{Function, FunctionSignature, FunctionAddress, FunctionStorage};
use crate::model::verifier::{Verifier, VerifyError};
use crate::model::typesystem::TypeStorage;
use crate::model::binder::Binder;
use crate::vm::Execution;
use crate::model::class::{Class};
use crate::optimization::register_allocation::RegisterAllocationSettings;

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionEngineError {
    Verify(VerifyError),
    NoMainFunction,
    NoMainFunctionCompiled,
    Runtime(RuntimeError),
    Other(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    NullReference,
    ArrayCreate,
    ArrayBounds
}

pub type ExecutionEngineResult<T> = Result<T, ExecutionEngineError>;

pub struct ExecutionEngine {
    compiler: JitCompiler,
    binder: Binder,
    pub runtime_error: RuntimeErrorManager
}

impl ExecutionEngine {
    pub fn new() -> ExecutionEngine {
        let mut jit_settings = JitSettings {
            register_allocate: true,
            register_allocation: RegisterAllocationSettings { num_int_registers: 2, num_float_registers: 2 }
        };

        let mut test_profile = std::env::var("TEST_PROFILE");
        // test_profile = Ok("4".to_owned());
        // test_profile = Ok("2".to_owned());

        if let Ok(test_profile) = test_profile {
            if test_profile == "2" {
                jit_settings = JitSettings {
                    register_allocate: false,
                    register_allocation: RegisterAllocationSettings { num_int_registers: 0, num_float_registers: 0 }
                };
            } else if test_profile == "3" {
                jit_settings = JitSettings {
                    register_allocate: true,
                    register_allocation: RegisterAllocationSettings { num_int_registers: 0, num_float_registers: 0 }
                };
            } else if test_profile == "4" {
                jit_settings = JitSettings {
                    register_allocate: true,
                    register_allocation: RegisterAllocationSettings { num_int_registers: 1, num_float_registers: 1 }
                };
            } else if test_profile == "5" {
                jit_settings = JitSettings {
                    register_allocate: true,
                    register_allocation: RegisterAllocationSettings { num_int_registers: 3, num_float_registers: 3 }
                };
            }
        }

        ExecutionEngine {
            compiler: JitCompiler::new(jit_settings),
            binder: Binder::new(),
            runtime_error: RuntimeErrorManager::new()
        }
    }

    pub fn create_execution(&mut self,
                            type_storage: &mut TypeStorage,
                            function_storage: &mut FunctionStorage) -> ExecutionEngineResult<Execution> {
        self.compile_functions(type_storage, function_storage)?;
        self.compiler.resolve_calls_and_branches(&self.binder);

        let address = self.get_entrypoint()?;
        let entrypoint = unsafe { std::mem::transmute(address) };
        Ok(Execution::new(entrypoint))
    }

    pub fn take_runtime_error(&mut self) -> Option<RuntimeError> {
        self.runtime_error.has_error.take()
    }

    fn compile_functions(&mut self,
                         type_storage: &mut TypeStorage,
                         function_storage: &mut FunctionStorage) -> ExecutionEngineResult<()> {
        for function in function_storage.functions_mut() {
            let mut verifier = Verifier::new(&self.binder, type_storage, function);
            verifier.verify().map_err(|err| ExecutionEngineError::Verify(err))?;
            self.compiler.compile_function(&mut self.binder, type_storage, function);
        }

        Ok(())
    }

    fn get_entrypoint(&self) -> ExecutionEngineResult<FunctionAddress> {
        self.binder
            .get(&FunctionSignature::new("main".to_owned(), Vec::new()))
            .ok_or(ExecutionEngineError::NoMainFunction)?
            .address()
            .ok_or(ExecutionEngineError::NoMainFunctionCompiled)
    }

    pub fn compiler(&self) -> &JitCompiler {
        &self.compiler
    }

    pub fn binder_mut(&mut self) -> &mut Binder {
        &mut self.binder
    }
}

pub struct RuntimeErrorManager {
    pub has_error: Option<RuntimeError>,
    pub return_address: u64,
    pub base_pointer: u64,
    pub stack_pointer: u64
}

impl RuntimeErrorManager {
    pub fn new() -> RuntimeErrorManager {
        RuntimeErrorManager {
            has_error: None,
            return_address: 0,
            base_pointer: 0,
            stack_pointer: 0
        }
    }
}