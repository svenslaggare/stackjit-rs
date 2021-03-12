pub mod linear_scan;

use std::collections::HashMap;

use crate::ir::HardwareRegister;
use crate::analysis::liveness::LiveInterval;
use crate::ir::mid::VirtualRegister;
use crate::model::typesystem::Type;
use iced_x86::OpCodeOperandKind::al;

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

    pub fn get_register(&self, register: &VirtualRegister) -> &AllocatedRegister {
        &self.registers[register]
    }

    pub fn alive_registers_at(&self, instruction_index: usize) -> Vec<HardwareRegister> {
        self.registers
            .iter()
            .filter(|(_, allocation)| allocation.hardware_register().is_some())
            .filter(|(_, allocation)| {
                let interval = allocation.interval();
                instruction_index >= interval.start && instruction_index <= interval.end
            })
            .map(|(register, allocation)| allocation.hardware_register().unwrap())
            .collect()
    }
}