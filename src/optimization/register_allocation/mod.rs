use std::collections::HashMap;

use crate::analysis::{VirtualRegister, VirtualRegisterType};
use crate::analysis::liveness::LiveInterval;
use crate::compiler::ir::HardwareRegister;
use crate::mir::RegisterMIR;
use crate::model::typesystem::Type;

pub mod linear_scan;

#[derive(Debug, Clone)]
pub enum AllocatedRegister {
    Hardware { register: HardwareRegister, live_interval: LiveInterval },
    Stack { live_interval: LiveInterval }
}

impl AllocatedRegister {
    pub fn interval(&self) -> &LiveInterval {
        match self {
            AllocatedRegister::Hardware { live_interval, .. } => live_interval,
            AllocatedRegister::Stack { live_interval, .. } => live_interval
        }
    }

    pub fn is_stack(&self) -> bool {
        match self {
            AllocatedRegister::Hardware { .. } => false,
            AllocatedRegister::Stack { .. } => true
        }
    }

    pub fn hardware_register(&self) -> Option<HardwareRegister> {
        match self {
            AllocatedRegister::Hardware { register, .. } => Some(register.clone()),
            AllocatedRegister::Stack { .. } => None
        }
    }
}

pub struct RegisterAllocation {
    registers: HashMap<VirtualRegister, AllocatedRegister>
}

impl RegisterAllocation {
    pub fn new(allocated: HashMap<LiveInterval, u32>, spilled: Vec<LiveInterval>) -> RegisterAllocation {
        let mut registers = HashMap::new();
        for (live_interval, register_number) in allocated {
            let register = match &live_interval.register.register_type {
                VirtualRegisterType::Int => HardwareRegister::Int(register_number),
                VirtualRegisterType::Float => HardwareRegister::Float(register_number)
            };

            registers.insert(live_interval.register.clone(), AllocatedRegister::Hardware { register, live_interval });
        }

        for live_interval in spilled {
            registers.insert(live_interval.register.clone(), AllocatedRegister::Stack { live_interval });
        }

        RegisterAllocation {
            registers
        }
    }

    pub fn num_allocated_registers(&self) -> usize {
        self.registers.values().filter(|register| register.hardware_register().is_some()).count()
    }

    pub fn num_spilled_registers(&self) -> usize {
        self.registers.values().filter(|register| register.hardware_register().is_none()).count()
    }

    pub fn get_register(&self, register: &RegisterMIR) -> &AllocatedRegister {
        &self.registers[&VirtualRegister::from(register)]
    }

    pub fn alive_registers_at(&self, instruction_index: usize) -> Vec<(VirtualRegister, HardwareRegister)> {
        self.registers
            .iter()
            .filter(|(_, allocation)| allocation.hardware_register().is_some())
            .filter(|(_, allocation)| {
                let interval = allocation.interval();
                instruction_index >= interval.start && instruction_index <= interval.end
            })
            .map(|(register, allocation)| (register.clone(), allocation.hardware_register().unwrap()))
            .collect()
    }

    pub fn alive_hardware_registers_at(&self, instruction_index: usize) -> Vec<HardwareRegister> {
        self.alive_registers_at(instruction_index).iter().map(|(_, register)| register.clone()).collect::<Vec<_>>()
    }
}