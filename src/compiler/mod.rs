use std::collections::HashMap;

pub mod code_generator;
pub mod allocator;
pub mod stack_layout;
pub mod jit;
pub mod calling_conventions;
pub mod error_handling;

use crate::ir::{BranchLabel, HardwareRegisterExplicit, InstructionIR};
use crate::model::function::{Function, FunctionSignature};

pub struct FunctionCompilationData {
    pub unresolved_function_calls: Vec<UnresolvedFunctionCall>,
    pub branch_targets: HashMap<BranchLabel, usize>,
    pub unresolved_branches: HashMap<usize, (BranchLabel, usize)>,
    pub unresolved_native_branches: HashMap<usize, usize>
}

impl FunctionCompilationData {
    pub fn new() -> FunctionCompilationData {
        FunctionCompilationData {
            unresolved_function_calls: Vec::new(),
            unresolved_branches: HashMap::new(),
            branch_targets: HashMap::new(),
            unresolved_native_branches: HashMap::new()
        }
    }
}

pub enum FunctionCallType {
    Relative,
    Absolute
}

pub struct UnresolvedFunctionCall {
    pub call_type: FunctionCallType,
    pub call_offset: usize,
    pub signature: FunctionSignature
}
