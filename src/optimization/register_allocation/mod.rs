pub mod linear_scan;

use std::collections::HashMap;

use crate::ir::low::HardwareRegister;
use crate::analysis::liveness::LiveInterval;
use crate::ir::mid::VirtualRegister;
use crate::model::typesystem::Type;

#[derive(Debug, Clone)]
pub enum AllocatedRegister {
    Hardware { register: HardwareRegister, live_interval: LiveInterval },
    Stack { stack_index: usize, live_interval: LiveInterval }
}

impl AllocatedRegister {
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
            let register = match &live_interval.register.value_type {
                Type::Float32 => HardwareRegister::Float(register_number),
                _ => HardwareRegister::Int(register_number)
            };

            registers.insert(live_interval.register.clone(), AllocatedRegister::Hardware { register, live_interval });
        }

        let mut stack_index = 0;
        for live_interval in spilled {
            registers.insert(live_interval.register.clone(), AllocatedRegister::Stack { live_interval, stack_index });
            stack_index += 1;
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
}