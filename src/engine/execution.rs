use crate::compiler::jit::{JitCompiler};
use crate::model::function::{Function, FunctionSignature, FunctionAddress};
use crate::model::verifier::{Verifier, VerifyError};
use crate::engine::binder::Binder;
use crate::model::typesystem::TypeStorage;
use crate::vm::{Execution};

#[derive(Debug)]
pub enum ExecutionEngineError {
    Verify(VerifyError),
    NoMainFunction,
    NoMainFunctionCompiled
}

pub type ExecutionEngineResult<T> = Result<T, ExecutionEngineError>;

pub struct ExecutionEngine {
    compiler: JitCompiler,
    binder: Binder,
    functions: Vec<Function>
}

impl ExecutionEngine {
    pub fn new() -> ExecutionEngine {
        ExecutionEngine {
            compiler: JitCompiler::new(),
            binder: Binder::new(),
            functions: Vec::new()
        }
    }

    pub fn add_function(&mut self, function: Function) -> ExecutionEngineResult<()> {
        self.binder.define(function.definition().clone());
        self.functions.push(function);
        Ok(())
    }

    pub fn create_execution(&mut self, type_storage: &mut TypeStorage) -> ExecutionEngineResult<Execution> {
        self.compile_functions(type_storage)?;
        self.compiler.resolve_calls_and_branches(&self.binder);

        let address = self.get_entrypoint()?;
        let entrypoint = unsafe { std::mem::transmute(address) };
        Ok(Execution::new(entrypoint))
    }

    fn compile_functions(&mut self, type_storage: &mut TypeStorage) -> ExecutionEngineResult<()> {
        for function in &mut self.functions {
            let mut verifier = Verifier::new(&self.binder, function);
            verifier.verify().map_err(|err| ExecutionEngineError::Verify(err))?;
            self.compiler.compile_function(&mut self.binder, type_storage, function);
        }

        Ok(())
    }

    fn get_entrypoint(&self) -> ExecutionEngineResult<FunctionAddress> {
        self.binder
            .get(&FunctionSignature { name: "main".to_owned(), parameters: Vec::new() })
            .ok_or(ExecutionEngineError::NoMainFunction)?
            .address()
            .ok_or(ExecutionEngineError::NoMainFunctionCompiled)
    }

    pub fn binder(&self) -> &Binder {
        &self.binder
    }

    pub fn binder_mut(&mut self) -> &mut Binder {
        &mut self.binder
    }
}