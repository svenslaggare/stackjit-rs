use std::collections::BTreeSet;

use crate::mir::RegisterMIR;
use crate::compiler::ir::{InstructionIR, HardwareRegister, HardwareRegisterExplicit, Variable};
use crate::optimization::register_allocation::{AllocatedRegister, RegisterAllocation};
use crate::analysis::VirtualRegister;
use crate::compiler::stack_layout;
use crate::model::function::Function;

pub trait AllocatedCompilerHelpers {
    fn function(&self) -> &Function;
    fn register_allocation(&self) -> &RegisterAllocation;
    fn instructions(&mut self) -> &mut Vec<InstructionIR>;

    fn push_alive_registers(&mut self, instruction_index: usize) -> Vec<(VirtualRegister, HardwareRegister)> {
        let alive_registers = self.register_allocation().alive_registers_at(instruction_index);
        for (virtual_register, register) in &alive_registers {
            let destination_offset = self.get_virtual_register_stack_offset(virtual_register);
            self.instructions().push(InstructionIR::StoreFrameMemory(destination_offset, register.clone()));
        }

        alive_registers
    }

    fn pop_alive_registers(&mut self,
                           alive_registers: &Vec<(VirtualRegister, HardwareRegister)>,
                           destination_register: Option<HardwareRegister>) {
        for (virtual_register, register) in alive_registers.iter().rev() {
            if let Some(destination_register) = destination_register.as_ref() {
                if destination_register != register {
                    let source_offset = self.get_virtual_register_stack_offset(&virtual_register);
                    self.instructions().push(InstructionIR::LoadFrameMemory(register.clone(), source_offset));
                } else {
                    // The assign register will have the return value as value, so don't pop to a register.
                }
            } else {
                let source_offset = self.get_virtual_register_stack_offset(&virtual_register);
                self.instructions().push(InstructionIR::LoadFrameMemory(register.clone(), source_offset));
            }
        }
    }

    fn move_register(&mut self,
                     destination: &RegisterMIR,
                     source: &RegisterMIR) {
        let destination_allocation = self.register_allocation().get_register(destination).clone();
        let source_allocation = self.register_allocation().get_register(source).clone();

        match (destination_allocation, source_allocation) {
            (AllocatedRegister::Hardware { register: destination_register, .. }, AllocatedRegister::Hardware { register: source_register, .. }) => {
                self.instructions().push(InstructionIR::Move(destination_register, source_register));
            }
            (AllocatedRegister::Hardware { register: destination_register, .. }, AllocatedRegister::Stack { .. }) => {
                let source_offset = self.get_register_stack_offset(source);
                self.instructions().push(InstructionIR::LoadFrameMemory(destination_register, source_offset));
            }
            (AllocatedRegister::Stack {  .. }, AllocatedRegister::Hardware { register: source_register, .. }) => {
                let destination_offset = self.get_register_stack_offset(destination);
                self.instructions().push(InstructionIR::StoreFrameMemory(destination_offset, source_register));
            }
            (AllocatedRegister::Stack { .. }, AllocatedRegister::Stack { .. }) => {
                let source_offset = self.get_register_stack_offset(source);
                let destination_offset = self.get_register_stack_offset(destination);
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, source_offset));
                self.instructions().push(InstructionIR::StoreFrameMemory(destination_offset, HardwareRegister::IntSpill));
            }
        }
    }

    fn move_from_hardware_register(&mut self,
                                   destination: &RegisterMIR,
                                   source: HardwareRegister) {
        let destination_allocation = self.register_allocation().get_register(destination).clone();
        match destination_allocation.hardware_register() {
            Some(destination_register) => {
                self.instructions().push(InstructionIR::Move(destination_register, source));
            }
            None => {
                let destination_offset = self.get_register_stack_offset(destination);
                self.instructions().push(InstructionIR::StoreFrameMemory(destination_offset, source));
            }
        }
    }

    fn move_to_hardware_register(&mut self,
                                 destination: HardwareRegister,
                                 source: &RegisterMIR) {
        let source_allocation = self.register_allocation().get_register(source).clone();
        match source_allocation.hardware_register() {
            Some(source_register) => {
                self.instructions().push(InstructionIR::Move(destination, source_register));
            }
            None => {
                let source_offset = self.get_register_stack_offset(source);
                self.instructions().push(InstructionIR::LoadFrameMemory(destination, source_offset));
            }
        }
    }

    fn move_from_variable(&mut self,
                          destination: &RegisterMIR,
                          source: &Variable) {
        let destination_allocation = self.register_allocation().get_register(destination).clone();
        match destination_allocation.hardware_register() {
            Some(destination_register) => {
                source.move_to_register(destination_register, self.instructions());
            }
            None => {
                let destination_offset = self.get_register_stack_offset(destination);
                source.move_to_stack_frame(destination_offset, self.instructions());
            }
        }
    }

    fn binary_operator_with_destination<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        let operand2_allocation = self.register_allocation().get_register(operand2).clone();
        let operand2_offset = self.get_register_stack_offset(operand2);

        if destination == operand1 {
            let destination_allocation = self.register_allocation().get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);

            self.binary_operator_internal(
                (destination_allocation.hardware_register(), destination_offset),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
                mem_reg
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
            self.binary_operator_internal(
                (Some(HardwareRegister::IntSpill), 0),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
                mem_reg
            );
            self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
        }
    }

    fn binary_operator<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        self.binary_operator_internal(
            (self.register_allocation().get_register(operand1).hardware_register(), self.get_register_stack_offset(operand1)),
            (self.register_allocation().get_register(operand2).hardware_register(), self.get_register_stack_offset(operand2)),
            reg_reg,
            reg_mem,
            mem_reg
        );
    }

    fn binary_operator_internal<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F3: Fn(&mut Vec<InstructionIR>, i32, HardwareRegister)
    >(&mut self,
      operand1: (Option<HardwareRegister>, i32),
      operand2: (Option<HardwareRegister>, i32),
      reg_reg: F1,
      reg_mem: F2,
      mem_reg: F3) {
        let (operand1_allocation, operand1_offset) = operand1;
        let (operand2_allocation, operand2_offset) = operand2;

        match (operand1_allocation, operand2_allocation) {
            (Some(operand1_register), Some(operand2_register)) => {
                reg_reg(&mut self.instructions(), operand1_register, operand2_register);
            }
            (Some(operand1_register), None) => {
                reg_mem(&mut self.instructions(), operand1_register, operand2_offset);
            }
            (None, Some(operand2_register)) => {
                mem_reg(&mut self.instructions(), operand1_offset, operand2_register);
            }
            (None, None) => {
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, operand2_offset));
                mem_reg(&mut self.instructions(), operand1_offset, HardwareRegister::IntSpill);
            }
        }
    }

    fn binary_operator_no_memory_store_with_destination<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2) {
        let operand2_allocation = self.register_allocation().get_register(operand2).clone();
        let operand2_offset = self.get_register_stack_offset(operand2);

        if destination == operand1 {
            let destination_allocation = self.register_allocation().get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);

            self.binary_operator_no_memory_store_internal(
                (destination_allocation.hardware_register(), destination_offset),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
            self.binary_operator_no_memory_store_internal(
                (Some(HardwareRegister::IntSpill), 0),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
            self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
        }
    }

    fn binary_operator_no_memory_store_internal<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: (Option<HardwareRegister>, i32),
      operand2: (Option<HardwareRegister>, i32),
      reg_reg: F1,
      reg_mem: F2) {
        let (operand1_allocation, operand1_offset) = operand1;
        let (operand2_allocation, operand2_offset) = operand2;

        match (operand1_allocation, operand2_allocation) {
            (Some(operand1_register), Some(operand2_register)) => {
                reg_reg(&mut self.instructions(), operand1_register, operand2_register);
            }
            (Some(operand1_register), None) => {
                reg_mem(&mut self.instructions(), operand1_register, operand2_offset);
            }
            (None, Some(operand2_register)) => {
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, operand1_offset));
                reg_reg(&mut self.instructions(), HardwareRegister::IntSpill, operand2_register);
                self.instructions().push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::IntSpill));
            }
            (None, None) => {
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::IntSpill, operand1_offset));
                reg_mem(&mut self.instructions(), HardwareRegister::IntSpill, operand2_offset);
                self.instructions().push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::IntSpill));
            }
        }
    }

    fn binary_operator_with_constant_and_destination<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32),
        F2: Fn(&mut Vec<InstructionIR>, i32, i32)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: i32,
      reg_constant: F1,
      mem_constant: F2) {
        let handle = |instructions: &mut Vec<InstructionIR>, operand1: (Option<HardwareRegister>, i32), operand2: i32| {
            let (operand1_allocation, operand1_offset) = operand1;

            match operand1_allocation {
                Some(operand1_register) => {
                    reg_constant(instructions, operand1_register, operand2);
                }
                None => {
                    mem_constant(instructions, operand1_offset, operand2);
                }
            }
        };

        if destination == operand1 {
            let destination_allocation = self.register_allocation().get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);
            handle(
                &mut self.instructions(),
                (destination_allocation.hardware_register(), destination_offset),
                operand2
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::IntSpill, operand1);
            handle(
                &mut self.instructions(),
                (Some(HardwareRegister::IntSpill), 0),
                operand2
            );
            self.move_from_hardware_register(destination, HardwareRegister::IntSpill);
        }
    }

    fn binary_operator_with_destination_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      destination: &RegisterMIR,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2) {
        let operand2_allocation = self.register_allocation().get_register(operand2).clone();
        let operand2_offset = self.get_register_stack_offset(operand2);

        if destination == operand1 {
            let destination_allocation = self.register_allocation().get_register(destination).clone();
            let destination_offset = self.get_register_stack_offset(destination);

            self.binary_operator_internal_f32(
                (destination_allocation.hardware_register(), destination_offset),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
        } else {
            self.move_to_hardware_register(HardwareRegister::FloatSpill, operand1);
            self.binary_operator_internal_f32(
                (Some(HardwareRegister::FloatSpill), 0),
                (operand2_allocation.hardware_register(), operand2_offset),
                reg_reg,
                reg_mem,
            );
            self.move_from_hardware_register(destination, HardwareRegister::FloatSpill);
        }
    }

    fn binary_operator_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: &RegisterMIR,
      operand2: &RegisterMIR,
      reg_reg: F1,
      reg_mem: F2) {
        self.binary_operator_internal_f32(
            (self.register_allocation().get_register(operand1).hardware_register(), self.get_register_stack_offset(operand1)),
            (self.register_allocation().get_register(operand2).hardware_register(), self.get_register_stack_offset(operand2)),
            reg_reg,
            reg_mem
        );
    }

    fn binary_operator_internal_f32<
        F1: Fn(&mut Vec<InstructionIR>, HardwareRegister, HardwareRegister),
        F2: Fn(&mut Vec<InstructionIR>, HardwareRegister, i32)
    >(&mut self,
      operand1: (Option<HardwareRegister>, i32),
      operand2: (Option<HardwareRegister>, i32),
      reg_reg: F1,
      reg_mem: F2) {
        let (operand1_allocation, operand1_offset) = operand1;
        let (operand2_allocation, operand2_offset) = operand2;

        match (operand1_allocation, operand2_allocation) {
            (Some(operand1_register), Some(operand2_register)) => {
                reg_reg(self.instructions(), operand1_register, operand2_register);
            }
            (Some(operand1_register), None) => {
                reg_mem(self.instructions(), operand1_register, operand2_offset);
            }
            (None, Some(operand2_register)) => {
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_reg(self.instructions(), HardwareRegister::FloatSpill, operand2_register);
                self.instructions().push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
            (None, None) => {
                self.instructions().push(InstructionIR::LoadFrameMemory(HardwareRegister::FloatSpill, operand1_offset));
                reg_mem(self.instructions(), HardwareRegister::FloatSpill, operand2_offset);
                self.instructions().push(InstructionIR::StoreFrameMemory(operand1_offset, HardwareRegister::FloatSpill));
            }
        }
    }

    fn get_virtual_register_stack_offset(&self, register: &VirtualRegister) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function(), register.number)
    }

    fn get_register_stack_offset(&self, register: &RegisterMIR) -> i32 {
        stack_layout::virtual_register_stack_offset(self.function(), register.number)
    }
}

pub struct TempRegisters {
    alive_registers: Vec<HardwareRegister>,
    int_registers: BTreeSet<HardwareRegister>,
    float_registers: BTreeSet<HardwareRegister>,
    saved_registers: Vec<HardwareRegister>,
    restored: bool
}

impl TempRegisters {
    pub fn new(register_allocation: &RegisterAllocation, instruction_index: usize) -> TempRegisters {
        let mut int_registers = BTreeSet::new();
        int_registers.insert(HardwareRegister::Int(5));
        int_registers.insert(HardwareRegister::Int(4));
        int_registers.insert(HardwareRegister::Int(3));
        // int_registers.insert(HardwareRegister::Int(0));
        // int_registers.insert(HardwareRegister::Int(1));
        // int_registers.insert(HardwareRegister::Int(2));

        let mut float_registers = BTreeSet::new();
        float_registers.insert(HardwareRegister::Float(4));
        float_registers.insert(HardwareRegister::Float(3));

        TempRegisters {
            alive_registers: register_allocation.alive_hardware_registers_at(instruction_index),
            int_registers,
            float_registers,
            saved_registers: Vec::new(),
            restored: false
        }
    }

    pub fn try_remove(&mut self, register_allocation: &RegisterAllocation, register: &RegisterMIR) {
        if let Some(register) = register_allocation.get_register(register).hardware_register() {
            self.int_registers.remove(&register);
            self.float_registers.remove(&register);
        }
    }

    pub fn get(&mut self,
               register_allocation: &RegisterAllocation,
               function: &Function,
               instructions: &mut Vec<InstructionIR>,
               register: &RegisterMIR) -> HardwareRegister {
        self.get_with_status(register_allocation, function, instructions, register).0
    }

    pub fn get_with_status(&mut self,
                           register_allocation: &RegisterAllocation,
                           function: &Function,
                           instructions: &mut Vec<InstructionIR>,
                           register: &RegisterMIR) -> (HardwareRegister, bool, bool) {
        let (is_stack, hardware_register) = self.get_raw(register_allocation, register);
        let alive = self.push_if_alive(function, instructions, register, &hardware_register, is_stack);

        if alive {
            self.saved_registers.push(hardware_register);
        }

        (hardware_register, is_stack, alive)
    }

    pub fn get_and_try_remove<const N: usize>(&mut self,
                                              register_allocation: &RegisterAllocation,
                                              function: &Function,
                                              instructions: &mut Vec<InstructionIR>,
                                              registers: [&RegisterMIR; N]) -> [HardwareRegister; N] {
        for register in &registers {
            self.try_remove(register_allocation, register);
        }

        let mut results: [HardwareRegister; N] = [HardwareRegister::Int(0); N];
        for (index, register) in registers.iter().enumerate() {
            results[index] = self.get(register_allocation, function, instructions, register);
        }

        results
    }

    fn get_raw(&mut self,
               register_allocation: &RegisterAllocation,
               register: &RegisterMIR) -> (bool, HardwareRegister) {
        match register_allocation.get_register(register).hardware_register() {
            Some(register) => (false, register.clone()),
            None if register.value_type.is_float() => {
                let register = self.float_registers.iter().rev().next().unwrap().clone();
                self.float_registers.remove(&register);
                (true, register)
            }
            None => {
                let register = self.int_registers.iter().rev().next().unwrap().clone();
                self.int_registers.remove(&register);
                (true, register)
            }
        }
    }

    fn push_if_alive(&mut self,
                     function: &Function,
                     instructions: &mut Vec<InstructionIR>,
                     register_ir: &RegisterMIR,
                     register: &HardwareRegister,
                     is_stack: bool) -> bool {
        let alive = if is_stack && self.alive_registers.contains(&register) {
            instructions.push(InstructionIR::Push(register.clone()));
            true
        } else {
            false
        };

        if is_stack {
            let source_offset = stack_layout::virtual_register_stack_offset(function, register_ir.number);
            instructions.push(InstructionIR::LoadFrameMemory(register.clone(), source_offset));
        }

        alive
    }

    pub fn clear(mut self) {
        self.saved_registers.clear();
        self.restored = true;
    }

    pub fn done(mut self, instructions: &mut Vec<InstructionIR>) {
        for register in std::mem::take(&mut self.saved_registers).into_iter().rev() {
            instructions.push(InstructionIR::Pop(register));
        }

        self.restored = true;
    }
}

impl Drop for TempRegisters {
    fn drop(&mut self) {
        if !self.restored {
            panic!("registers not restored.");
        }
    }
}