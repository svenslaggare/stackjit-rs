use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::iter::FromIterator;

use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::analysis::liveness::{compute_liveness, LiveInterval};
use crate::engine::binder::Binder;
use crate::ir::branches;
use crate::ir::mid::{InstructionMIR, VirtualRegister};
use crate::ir::compiler::InstructionMIRCompiler;
use crate::model::function::{Function, FunctionDefinition};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::model::verifier::Verifier;
use crate::optimization::register_allocation::{AllocatedRegister, RegisterAllocation};

pub struct Settings {
    pub num_int_registers: usize,
    pub num_float_registers: usize
}

pub fn allocate(live_intervals: &Vec<LiveInterval>, settings: &Settings) -> RegisterAllocation {
    let mut allocated_registers = HashMap::new();
    let mut spilled_registers = Vec::new();
    let mut free_registers = FreeRegisters::new(settings);

    let mut live_intervals = live_intervals.clone();
    live_intervals.sort_by_key(|interval| interval.start);

    let mut active = BTreeSet::<LiveIntervalByEndPoint>::new();
    for interval in &live_intervals {
        expire_old_intervals(
            &mut allocated_registers,
            &mut active,
            &mut free_registers,
            interval
        );

        let register_type = &interval.register.value_type;

        let active_of_same_type = active
            .iter()
            .map(|register| &register.0.register.value_type == register_type)
            .count();

        if active_of_same_type == free_registers.max_for_type(register_type) {
            split_at_interval(
                &mut allocated_registers,
                &mut spilled_registers,
                &mut active,
                interval
            );
        } else {
            let free_register = free_registers.get_free_register(register_type);
            allocated_registers.insert(interval.clone(), free_register);
            active.insert(LiveIntervalByEndPoint(interval.clone()));
        }
    }

    RegisterAllocation::new(allocated_registers, spilled_registers)
}

struct FreeRegisters {
    int_registers: BTreeSet<u32>,
    max_int: usize,
    float_registers: BTreeSet<u32>,
    max_float: usize,
}

impl FreeRegisters {
    pub fn new(settings: &Settings) -> FreeRegisters {
        FreeRegisters {
            int_registers: BTreeSet::from_iter(0 as u32..settings.num_int_registers as u32),
            max_int: settings.num_int_registers,
            float_registers: BTreeSet::from_iter(0 as u32..settings.num_float_registers as u32),
            max_float: settings.num_float_registers
        }
    }

    pub fn max_for_type(&self, value_type: &Type) -> usize {
        match value_type {
            Type::Float32 => self.max_float,
            _ => self.max_int
        }
    }

    pub fn for_type(&self, value_type: &Type) -> &BTreeSet<u32> {
        match value_type {
            Type::Float32 => &self.float_registers,
            _ => &self.int_registers
        }
    }

    pub fn for_type_mut(&mut self, value_type: &Type) -> &mut BTreeSet<u32> {
        match value_type {
            Type::Float32 => &mut self.float_registers,
            _ => &mut self.int_registers
        }
    }

    pub fn get_free_register(&mut self, value_type: &Type) -> u32 {
        let registers = self.for_type_mut(value_type);
        let free_register = *registers.iter().next().unwrap();
        registers.remove(&free_register);
        free_register
    }
}

fn expire_old_intervals(allocated_registers: &mut HashMap<LiveInterval, u32>,
                        active: &mut BTreeSet<LiveIntervalByEndPoint>,
                        free_registers: &mut FreeRegisters,
                        current_interval: &LiveInterval) {
    let mut to_remove = Vec::new();
    for interval in active.iter() {
        if interval.0.end >= current_interval.start {
            break;
        }

        to_remove.push(interval.clone());

        free_registers
            .for_type_mut(&interval.0.register.value_type)
            .insert(allocated_registers[&interval.0]);
    }

    for interval in to_remove {
        active.remove(&interval);
    }
}

fn split_at_interval(allocated_registers: &mut HashMap<LiveInterval, u32>,
                     spilled_registers: &mut Vec<LiveInterval>,
                     active: &mut BTreeSet<LiveIntervalByEndPoint>,
                     current_interval: &LiveInterval) {
    let spill = active.iter()
        .filter(|register| register.0.register.value_type == current_interval.register.value_type)
        .last()
        .unwrap()
        .clone();

    if spill.0.end > current_interval.end {
        allocated_registers.insert(current_interval.clone(), allocated_registers[&spill.0].clone());

        spilled_registers.push(spill.0.clone());
        allocated_registers.remove(&spill.0);

        active.remove(&spill);
        active.insert(LiveIntervalByEndPoint(current_interval.clone()));
    } else {
        spilled_registers.push(current_interval.clone());
        allocated_registers.remove(current_interval);
    }
}

#[derive(Clone)]
struct LiveIntervalByEndPoint(LiveInterval);

impl PartialEq for LiveIntervalByEndPoint {
    fn eq(&self, other: &Self) -> bool {
        self.0.register == other.0.register
        && self.0.start == other.0.start
        && self.0.end == other.0.end
    }
}

impl Eq for LiveIntervalByEndPoint {}

impl PartialOrd for LiveIntervalByEndPoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut result = self.0.end.partial_cmp(&other.0.end)?;
        if result == Ordering::Equal {
            result = self.0.start.partial_cmp(&other.0.start)?;

            if result == Ordering::Equal {
                return self.0.register.partial_cmp(&other.0.register);
            }
        }

        Some(result)
    }
}

impl Ord for LiveIntervalByEndPoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(&other).unwrap()
    }
}

fn analyze(instructions: &Vec<InstructionMIR>) -> (Vec<BasicBlock>, ControlFlowGraph, Vec<LiveInterval>) {
    let blocks = BasicBlock::create_blocks(&instructions);
    let control_flow_graph = ControlFlowGraph::new(
        &instructions,
        &blocks,
        &branches::create_label_mapping(&instructions)
    );

    let live_intervals = compute_liveness(&instructions, &blocks, &control_flow_graph);

    (blocks, control_flow_graph, live_intervals)
}

fn print_allocation(instructions: &Vec<InstructionMIR>, live_intervals: &Vec<LiveInterval>, allocation: &RegisterAllocation) {
    for (index, instruction) in instructions.iter().enumerate() {
        println!("{}: {:?}", index, instruction);
    }

    println!();

    for interval in live_intervals {
        println!("{:?}", interval);
    }

    println!();

    for (virtual_register, allocated_register) in &allocation.registers {
        println!("{:?}: {:?}", virtual_register, allocated_register);
    }
}

#[test]
fn test_allocate1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1),

            Instruction::LoadInt32(2),
            Instruction::Add,

            Instruction::LoadInt32(3),
            Instruction::Add,

            Instruction::LoadInt32(4),
            Instruction::Add,

            Instruction::LoadInt32(5),
            Instruction::Add,

            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let instructions = compiler.done().instructions;

    let (_, _, live_intervals) = analyze(&instructions);

    let allocation = allocate(
        &live_intervals,
        &Settings { num_int_registers: 1, num_float_registers: 0 }
    );

    assert_eq!(1, allocation.num_allocated_registers());
    assert_eq!(1, allocation.num_spilled_registers());

    print_allocation(&instructions, &live_intervals, &allocation);
}

#[test]
fn test_allocate2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32, Type::Int32],
        vec![
            Instruction::LoadInt32(40000),
            Instruction::StoreLocal(1),

            Instruction::LoadInt32(1337),
            Instruction::LoadInt32(4711),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(1),
            Instruction::Return
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let instructions = compiler.done().instructions;

    let (_, _, live_intervals) = analyze(&instructions);

    let allocation = allocate(
        &live_intervals,
        &Settings { num_int_registers: 1, num_float_registers: 0 }
    );

    assert_eq!(2, allocation.num_allocated_registers());
    assert_eq!(2, allocation.num_spilled_registers());

    print_allocation(&instructions, &live_intervals, &allocation);
}

#[test]
fn test_allocate3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Float32),
        vec![],
        vec![
            Instruction::LoadFloat32(1.0),

            Instruction::LoadFloat32(2.0),
            Instruction::Add,

            Instruction::LoadFloat32(3.0),
            Instruction::Add,

            Instruction::LoadFloat32(4.0),
            Instruction::Add,

            Instruction::LoadFloat32(5.0),
            Instruction::Add,

            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let instructions = compiler.done().instructions;

    let (_, _, live_intervals) = analyze(&instructions);

    let allocation = allocate(
        &live_intervals,
        &Settings { num_int_registers: 0, num_float_registers: 1 }
    );

    assert_eq!(1, allocation.num_allocated_registers());
    assert_eq!(1, allocation.num_spilled_registers());

    print_allocation(&instructions, &live_intervals, &allocation);
}

#[test]
fn test_allocate4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Float32),
        vec![Type::Float32, Type::Float32],
        vec![
            Instruction::LoadFloat32(40000.0),
            Instruction::StoreLocal(1),

            Instruction::LoadFloat32(1337.0),
            Instruction::LoadFloat32(4711.0),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(1),
            Instruction::Return
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let instructions = compiler.done().instructions;

    let (_, _, live_intervals) = analyze(&instructions);

    let allocation = allocate(
        &live_intervals,
        &Settings { num_int_registers: 0, num_float_registers: 1 }
    );

    assert_eq!(2, allocation.num_allocated_registers());
    assert_eq!(2, allocation.num_spilled_registers());

    print_allocation(&instructions, &live_intervals, &allocation);
}