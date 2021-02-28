use std::cell::RefCell;
use std::rc::Rc;
use std::ops::DerefMut;

use crate::engine::ExecutionEngine;
use crate::runtime::{Runtime};
use crate::model::typesystem::{TypeStorage};
use crate::engine::execution::{ExecutionEngineResult};
use crate::compiler::jit::EntryPoint;

pub struct VirtualMachine {
    pub engine: ExecutionEngine,
    pub runtime: Runtime,
    pub type_storage: TypeStorage
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {

        VirtualMachine {
            engine: ExecutionEngine::new(),
            runtime: Runtime::new(),
            type_storage: TypeStorage::new()
        }
    }

    pub fn prepare_execution(&mut self) -> ExecutionEngineResult<Execution> {
        self.engine.prepare_execution(&mut self.type_storage)
    }
}

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
        assign_vm(Rc::new(RefCell::new(virtual_machine)));
        Ok((self.entrypoint)())
    }
}

pub type VirtualMachineRef = Rc<RefCell<VirtualMachine>>;

pub fn assign_vm(virtual_machine: VirtualMachineRef) {
    VIRTUAL_MACHINE_INSTANCE.with(|vm_ref| {
        *vm_ref.borrow_mut() = Some(virtual_machine);
    });
}

pub fn get_vm<F: FnMut(&mut VirtualMachine) -> R, R>(mut f: F) -> R {
    VIRTUAL_MACHINE_INSTANCE.with(|vm_ref| {
        f(vm_ref.borrow().as_ref().unwrap().borrow_mut().deref_mut())
    })
}

thread_local!(static VIRTUAL_MACHINE_INSTANCE: RefCell<Option<VirtualMachineRef>> = RefCell::new(None));
