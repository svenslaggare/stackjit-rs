use std::cell::RefCell;

use crate::engine::ExecutionEngine;
use crate::model::typesystem::{TypeStorage};
use crate::engine::execution::{ExecutionEngineResult, ExecutionEngineError};
use crate::runtime::memory::manager::MemoryManager;

pub struct VirtualMachine {
    pub engine: ExecutionEngine,
    pub memory_manager: MemoryManager
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {
        VirtualMachine {
            engine: ExecutionEngine::new(),
            memory_manager: MemoryManager::new()
        }
    }

    pub fn execute(mut self) -> ExecutionEngineResult<i32> {
        self.create_execution()?.execute(self)
    }

    pub fn create_execution(&mut self) -> ExecutionEngineResult<Execution> {
        self.engine.create_execution()
    }
}

pub type EntryPoint = extern "C" fn() -> i32;

pub struct Execution {
    entrypoint: EntryPoint
}

impl Execution {
    pub fn new(entrypoint: EntryPoint) -> Execution {
        Execution {
            entrypoint,
        }
    }

    pub fn execute(&mut self, virtual_machine: VirtualMachine) -> ExecutionEngineResult<i32> {
        assign_vm(virtual_machine);
        let execution_result = (self.entrypoint)();
        let result = get_vm(|vm| {
            if let Some(err) = vm.engine.take_runtime_error() {
                Err(ExecutionEngineError::Runtime(err))
            } else {
                Ok(execution_result)
            }
        });

        result
    }
}

pub fn assign_vm(virtual_machine: VirtualMachine) {
    VIRTUAL_MACHINE_INSTANCE.with(|vm_ref| {
        *vm_ref.borrow_mut() = Some(virtual_machine);
    });
}

pub fn get_vm<F: FnMut(&mut VirtualMachine) -> R, R>(mut f: F) -> R {
    VIRTUAL_MACHINE_INSTANCE.with(|vm| {
        f(vm.borrow_mut().as_mut().unwrap())
    })
}

pub fn clear_vm() {
    VIRTUAL_MACHINE_INSTANCE.with(|vm_ref| {
        *vm_ref.borrow_mut() = None;
    });
}

thread_local!(static VIRTUAL_MACHINE_INSTANCE: RefCell<Option<VirtualMachine>> = RefCell::new(None));
