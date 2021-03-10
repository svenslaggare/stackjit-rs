use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::analysis::basic_block::{BasicBlock};
use crate::ir::mid::{InstructionMIRData, InstructionMIR};
use crate::ir::low::BranchLabel;
use crate::model::verifier::Verifier;
use crate::model::function::{Function, FunctionDefinition};
use crate::model::typesystem::Type;
use crate::model::instruction::Instruction;
use crate::engine::binder::Binder;
use crate::ir::mid::compiler::InstructionMIRCompiler;
use crate::ir::branches;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlFlowEdge {
    pub from: usize,
    pub to: usize
}

pub struct ControlFlowGraph {
    pub vertices: Vec<usize>,
    pub edges: HashMap<usize, HashSet<ControlFlowEdge>>,
    pub back_edges: HashMap<usize, HashSet<ControlFlowEdge>>
}

impl ControlFlowGraph {
    pub fn new(instructions: &Vec<InstructionMIR>,
               blocks: &Vec<BasicBlock>,
               branch_label_mapping: &HashMap<BranchLabel, usize>) -> ControlFlowGraph {
        let vertices = (0..blocks.len()).collect::<Vec<_>>();
        let mut edges = HashMap::new();
        let mut back_edges = HashMap::new();

        let mut start_offset_mapping = HashMap::new();
        for (block_index, block) in blocks.iter().enumerate() {
            start_offset_mapping.insert(block.start_offset, block_index);
        }

        let mut add_edge = |from, to| {
            edges.entry(from).or_insert_with(|| HashSet::new()).insert(ControlFlowEdge { from, to });
            back_edges.entry(to).or_insert_with(|| HashSet::new()).insert(ControlFlowEdge { from: to, to: from });
        };

        for (block_index, block) in blocks.iter().enumerate() {
            match &instructions[block.last()].data {
                InstructionMIRData::Branch(label) => {
                    let target_block_index = start_offset_mapping[&branch_label_mapping[label]];
                    add_edge(block_index, target_block_index);
                }
                InstructionMIRData::BranchCondition(_, _, label, _, _) => {
                    let target_block_index = start_offset_mapping[&branch_label_mapping[label]];
                    add_edge(block_index, target_block_index);
                    add_edge(block_index, start_offset_mapping[&(block.start_offset + block.instructions.len())]);
                }
                InstructionMIRData::Return(_) => {}
                _ => {
                    add_edge(block_index, start_offset_mapping[&(block.start_offset + block.instructions.len())]);
                }
            }
        }

        ControlFlowGraph {
            vertices,
            edges,
            back_edges
        }
    }

    pub fn print_graph(&self, instructions: &Vec<InstructionMIR>, blocks: &Vec<BasicBlock>) {
        for vertex_index in &self.vertices {
            let block = &blocks[*vertex_index];
            println!(
                "{} {}..{} {}",
                instructions[block.first()].name(),
                block.start_offset,
                instructions[block.last()].name(),
                block.start_offset + block.instructions.len() - 1
            );

            if let Some(edges) = self.edges.get(vertex_index) {
                for edge in edges {
                    let edge_block = &blocks[edge.to];
                    println!(
                        "\t{} {}..{} {}",
                        instructions[edge_block.first()].name(),
                        edge_block.start_offset,
                        instructions[edge_block.last()].name(),
                        edge_block.start_offset + edge_block.instructions.len() - 1
                    );
                }
            }
        }
    }
}

#[test]
fn test_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Int32],
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

    let mut binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let mut instructions = compiler.done().instructions;

    let blocks = BasicBlock::create_blocks(&instructions);
    let branch_label_mapping = branches::create_label_mapping(&instructions);

    let control_flow_graph = ControlFlowGraph::new(&instructions, &blocks, &branch_label_mapping);
    control_flow_graph.print_graph(&instructions, &blocks);

    assert_eq!(
        vec![ControlFlowEdge { from: 0, to: 1 }, ControlFlowEdge { from: 0, to: 2 }],
        extract_edges(&control_flow_graph, 0)
    );

    assert_eq!(
        vec![ControlFlowEdge { from: 1, to: 3 }],
        extract_edges(&control_flow_graph, 1)
    );

    assert_eq!(
        vec![ControlFlowEdge { from: 2, to: 3 }],
        extract_edges(&control_flow_graph, 2)
    );

    assert_eq!(
        Vec::<ControlFlowEdge>::new(),
        extract_edges(&control_flow_graph, 3)
    );
}

fn extract_edges(graph: &ControlFlowGraph, index: usize) -> Vec<ControlFlowEdge> {
    if let Some(edges) = graph.edges.get(&index) {
        let mut edges = Vec::from_iter(edges.iter().cloned());
        edges.sort_by_key(|edge| edge.to);
        edges
    } else {
        Vec::new()
    }
}