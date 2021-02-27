use crate::compiler::jit::{JitCompiler, EntryPoint};
use crate::model::function::{Function, FunctionSignature};
use crate::model::verifier::{Verifier, VerifyError};
use crate::compiler::binder::Binder;

#[derive(Debug)]
pub enum ExecutionEngineError {
    Verify(VerifyError),
    NoMainFunction
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

    pub fn add_function(&mut self, mut function: Function) -> ExecutionEngineResult<()> {
        self.binder.define(function.definition().clone());
        self.functions.push(function);
        Ok(())
    }

    pub fn prepare_execution(&mut self) -> ExecutionEngineResult<EntryPoint> {
        for function in &mut self.functions {
            let mut verifier = Verifier::new(&self.binder, function);
            verifier.verify().map_err(|err| ExecutionEngineError::Verify(err))?;
            self.compiler.compile_function(&mut self.binder, function);
        }

        self.compiler.resolve_calls(&self.binder);
        let address = self.binder
            .get(&FunctionSignature { name: "main".to_owned(), parameters: Vec::new() })
            .ok_or(ExecutionEngineError::NoMainFunction)?
            .address()
            .ok_or(ExecutionEngineError::NoMainFunction)?;

        Ok(unsafe { std::mem::transmute(address) })
    }

    pub fn binder(&self) -> &Binder {
        &self.binder
    }

    pub fn binder_mut(&mut self) -> &mut Binder {
        &mut self.binder
    }
}