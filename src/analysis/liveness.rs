use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::analysis::VirtualRegister;
use crate::model::binder::Binder;
use crate::mir::{branches, InstructionMIR, RegisterMIR};
use crate::mir::compiler::{InstructionMIRCompiler, MIRCompilationResult};
use crate::model::function::{Function, FunctionDeclaration};
use crate::model::instruction::Instruction;
use crate::model::typesystem::{TypeId, TypeStorage};
use crate::model::verifier::Verifier;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LiveInterval {
    pub start: usize,
    pub end: usize,
    pub register: VirtualRegister
}

pub fn compute_liveness(compilation_result: &MIRCompilationResult,
                        basic_blocks: &Vec<BasicBlock>,
                        control_flow_graph: &ControlFlowGraph) -> Vec<LiveInterval> {
    let instructions = &compilation_result.instructions;
    let locals_references = HashSet::<u32>::from_iter(
        compilation_result.local_virtual_registers
            .iter()
            .filter(|register| register.value_type.is_reference())
            .map(|register| register.number)
    );

    let mut live_intervals = Vec::new();
    let virtual_registers = get_virtual_registers(instructions, basic_blocks, control_flow_graph);
    let (use_sites, assign_sites) = get_register_usage(instructions, basic_blocks, control_flow_graph);

    for register in virtual_registers {
        if let Some(register_use_sites) = use_sites.get(&register) {
            let mut alive_at = HashSet::new();
            compute_liveness_for_register(
                instructions,
                basic_blocks,
                control_flow_graph,
                &register,
                register_use_sites,
                &mut alive_at
            );

            if locals_references.contains(&register.number) {
                alive_at.extend(0..instructions.len());
            }

            live_intervals.push(get_live_interval(&register, &alive_at));
        } else {
            //This mean that the register is not used. Atm, we do not remove write-only virtual registers.
            //So we need to compute the liveness information, else there won't exist any liveness information for the register.
            let mut alive_at = HashSet::new();
            for assign_site in &assign_sites[&register] {
                alive_at.insert(basic_blocks[assign_site.block_index].start_offset + assign_site.offset);
            }

            if locals_references.contains(&register.number) {
                alive_at.extend(0..instructions.len());
            }

            live_intervals.push(get_live_interval(&register, &alive_at));
        }
    }

    live_intervals
}

fn get_live_interval(register: &VirtualRegister, alive_at: &HashSet<usize>) -> LiveInterval {
    let mut start = usize::max_value();
    let mut end = 0;

    for &instruction_index in alive_at {
        start = start.min(instruction_index);
        end = end.max(instruction_index);
    }

    LiveInterval {
        start,
        end,
        register: register.clone()
    }
}

fn get_virtual_registers(instructions: &Vec<InstructionMIR>,
                         basic_blocks: &Vec<BasicBlock>,
                         control_flow_graph: &ControlFlowGraph) -> Vec<VirtualRegister> {
    let mut registers = HashSet::new();

    for &block_index in &control_flow_graph.vertices {
        for &block_offset in &basic_blocks[block_index].instructions {
            let instruction = &instructions[block_offset];
            if let Some(assign_register) = instruction.data.assign_virtual_register() {
                registers.insert(assign_register);
            }

            for register in instruction.data.use_virtual_registers() {
                registers.insert(register);
            }
        }
    }

    let mut registers = Vec::from_iter(registers.into_iter());
    registers.sort_by_key(|register| register.number);
    registers
}

fn compute_liveness_for_register(instructions: &Vec<InstructionMIR>,
                                 basic_blocks: &Vec<BasicBlock>,
                                 control_flow_graph: &ControlFlowGraph,
                                 register: &VirtualRegister,
                                 use_sites: &Vec<UsageSite>,
                                 alive_at: &mut HashSet<usize>) {
    for use_site in use_sites {
        compute_liveness_for_register_in_block(
            instructions,
            basic_blocks,
            control_flow_graph,
            use_site.block_index,
            use_site.offset,
            &mut HashSet::new(),
            register,
            alive_at
        );
    }
}

fn compute_liveness_for_register_in_block(instructions: &Vec<InstructionMIR>,
                                          basic_blocks: &Vec<BasicBlock>,
                                          control_flow_graph: &ControlFlowGraph,
                                          block_index: usize,
                                          start_offset: usize,
                                          visited: &mut HashSet<usize>,
                                          register: &VirtualRegister,
                                          alive_at: &mut HashSet<usize>) {
    if visited.contains(&block_index) {
        return;
    }

    visited.insert(block_index);
    let mut terminated = false;
    for i in (0..(start_offset + 1)).rev() {
        let instruction = &instructions[basic_blocks[block_index].instructions[i]];

        if let Some(assign_register) = instruction.data.assign_virtual_register() {
            if &assign_register == register && !instruction.data.use_virtual_registers().contains(&register) {
                alive_at.insert(basic_blocks[block_index].start_offset + i);
                terminated = true;
                break;
            }
        }

        alive_at.insert(i + basic_blocks[block_index].start_offset);
    }

    //If we have not terminated the search, search edges flowing backwards from the current block
    if !terminated {
        if let Some(edges) = control_flow_graph.back_edges.get(&block_index) {
            for edge in edges {
                compute_liveness_for_register_in_block(
                    instructions,
                    basic_blocks,
                    control_flow_graph,
                    edge.to,
                    basic_blocks[edge.to].instructions.len() - 1,
                    visited,
                    &register,
                    alive_at
                );
            }
        }
    }
}

struct UsageSite {
    block_index: usize,
    offset: usize
}

type UseSites = HashMap<VirtualRegister, Vec<UsageSite>>;
type AssignSites = HashMap<VirtualRegister, Vec<UsageSite>>;

fn get_register_usage(instructions: &Vec<InstructionMIR>,
                      basic_blocks: &Vec<BasicBlock>,
                      control_flow_graph: &ControlFlowGraph) -> (UseSites, AssignSites) {
    let mut use_sites = HashMap::new();
    let mut assign_sites = HashMap::new();

    for &block_index in &control_flow_graph.vertices {
        for (block_offset, &instruction_index) in basic_blocks[block_index].instructions.iter().enumerate() {
            let instruction = &instructions[instruction_index];

            if let Some(assign_register) = instruction.data.assign_virtual_register() {
                assign_sites.entry(assign_register).or_insert_with(|| Vec::new()).push(UsageSite {
                    block_index,
                    offset: block_offset
                });
            }

            for use_register in instruction.data.use_virtual_registers() {
                use_sites.entry(use_register).or_insert_with(|| Vec::new()).push(UsageSite {
                    block_index,
                    offset: block_offset
                });
            }
        }
    }

    (use_sites, assign_sites)
}

#[test]
fn test_liveness1() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
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
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let instructions = &compilation_result.instructions;

    let blocks = BasicBlock::create_blocks(&instructions);
    let control_flow_graph = ControlFlowGraph::new(&instructions, &blocks);

    let live_intervals = compute_liveness(&compilation_result, &blocks, &control_flow_graph);

    assert_eq!(2, live_intervals.len());

    assert_eq!(0, live_intervals[0].register.number);
    assert_eq!(0, live_intervals[0].start);
    assert_eq!(9, live_intervals[0].end);

    assert_eq!(1, live_intervals[1].register.number);
    assert_eq!(1, live_intervals[1].start);
    assert_eq!(8, live_intervals[1].end);


    for (index, instruction) in instructions.iter().enumerate() {
        println!("{}: {:?}", index, instruction);
    }

    println!();

    for interval in live_intervals {
        println!("{:?}", interval);
    }
}

#[test]
fn test_liveness2() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(3),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let instructions = &compilation_result.instructions;

    let blocks = BasicBlock::create_blocks(&instructions);
    let control_flow_graph = ControlFlowGraph::new(&instructions, &blocks);

    let live_intervals = compute_liveness(&compilation_result, &blocks, &control_flow_graph);

    assert_eq!(3, live_intervals.len());

    assert_eq!(0, live_intervals[0].register.number);
    assert_eq!(3, live_intervals[0].start);
    assert_eq!(3, live_intervals[0].end);

    assert_eq!(1, live_intervals[1].register.number);
    assert_eq!(0, live_intervals[1].start);
    assert_eq!(5, live_intervals[1].end);

    assert_eq!(2, live_intervals[2].register.number);
    assert_eq!(1, live_intervals[2].start);
    assert_eq!(2, live_intervals[2].end);

    for (index, instruction) in instructions.iter().enumerate() {
        println!("{}: {:?}", index, instruction);
    }

    println!();

    for interval in live_intervals {
        println!("{:?}", interval);
    }
}

#[test]
fn test_liveness3() {
    let mut function = Function::new(
        FunctionDeclaration::new_managed("test".to_owned(), vec![], TypeId::Int32),
        vec![TypeId::Int32],
        vec![
            Instruction::LoadInt32(1),
            Instruction::LoadInt32(2),
            Instruction::BranchNotEqual(6),

            Instruction::LoadInt32(1337),
            Instruction::StoreLocal(0),
            Instruction::Branch(8),

            Instruction::LoadInt32(4711),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    let type_storage = TypeStorage::new();
    Verifier::new(&binder, &type_storage, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let instructions = &compilation_result.instructions;

    let blocks = BasicBlock::create_blocks(&instructions);
    let control_flow_graph = ControlFlowGraph::new(&instructions, &blocks);

    let live_intervals = compute_liveness(&compilation_result, &blocks, &control_flow_graph);

    assert_eq!(3, live_intervals.len());

    assert_eq!(0, live_intervals[0].register.number);
    assert_eq!(4, live_intervals[0].start);
    assert_eq!(10, live_intervals[0].end);

    assert_eq!(1, live_intervals[1].register.number);
    assert_eq!(0, live_intervals[1].start);
    assert_eq!(11, live_intervals[1].end);

    assert_eq!(2, live_intervals[2].register.number);
    assert_eq!(1, live_intervals[2].start);
    assert_eq!(2, live_intervals[2].end);

    for (index, instruction) in instructions.iter().enumerate() {
        println!("{}: {:?}", index, instruction);
    }

    println!();

    for interval in live_intervals {
        println!("{:?}", interval);
    }
}