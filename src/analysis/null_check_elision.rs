use std::collections::{HashMap, HashSet};

use crate::ir::mid::{InstructionMIR, InstructionMIRData, VirtualRegister};
use crate::analysis::basic_block::BasicBlock;
use crate::analysis::control_flow_graph::ControlFlowGraph;
use crate::ir::compiler::{MIRCompilationResult, InstructionMIRCompiler};
use crate::model::function::{Function, FunctionDefinition};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;
use crate::engine::binder::Binder;
use crate::model::verifier::Verifier;
use crate::ir::branches;

pub type InstructionsRegisterNullStatus = Vec<HashMap<VirtualRegister, bool>>;

pub fn compute_null_check_elision(function: &Function,
                                  compilation_result: &MIRCompilationResult,
                                  basic_blocks: &Vec<BasicBlock>,
                                  control_flow_graph: &ControlFlowGraph) -> InstructionsRegisterNullStatus {
    // return compilation_result.instructions.iter().map(|_| HashMap::new()).collect();

    if basic_blocks.len() == 1 {
        compute_null_check_elision_for_block(function, compilation_result, &basic_blocks[0])
    } else {
        // compilation_result.instructions.iter().map(|_| HashMap::new()).collect()

        let mut basic_blocks_results = Vec::<InstructionsRegisterNullStatus>::new();
        let mut instruction_results: InstructionsRegisterNullStatus = compilation_result.instructions.iter().map(|_| HashMap::new()).collect();
        for (basic_block_index, basic_block) in basic_blocks.iter().enumerate() {
            let entry_points = &control_flow_graph.back_edges.get(&basic_block_index);

            let block_result = if let Some(entry_points) = entry_points {
                let mut potential_block_results = Vec::new();

                // Record all potential null status of blocks that jump into this block
                for entry_point_block_index in entry_points.iter().map(|edge| edge.to) {
                    if let Some(basic_blocks_result) = basic_blocks_results.get(entry_point_block_index) {
                        potential_block_results.push(compute_null_check_elision_for_block_internal(
                            function,
                            compilation_result,
                            basic_block,
                            basic_blocks_result.last().unwrap().clone()
                        ));
                    } else {
                        // Too complicated to analyze (cyclic jumps), all are nulls.
                        return compilation_result.instructions.iter().map(|_| HashMap::new()).collect();
                    }
                }

                // Conservative merge all the potential results into one final result
                merge_results(potential_block_results)
            } else {
                // Should only be the first block
                compute_null_check_elision_for_block(function, compilation_result, basic_block)
            };

            for (instruction_offset, instruction) in block_result.iter().enumerate() {
                let instruction_index = basic_block.instructions[instruction_offset];
                instruction_results[instruction_index] = instruction.clone();
            }

            basic_blocks_results.push(block_result);
        }

        instruction_results
    }
}

fn compute_null_check_elision_for_block(function: &Function,
                                        compilation_result: &MIRCompilationResult,
                                        basic_block: &BasicBlock) -> InstructionsRegisterNullStatus {
    let mut register_is_null = HashMap::new();
    for register in &compilation_result.local_virtual_registers {
        register_is_null.insert(register.clone(), true);
    }

    compute_null_check_elision_for_block_internal(function, compilation_result, basic_block, register_is_null)
}

fn compute_null_check_elision_for_block_internal(function: &Function,
                                                 compilation_result: &MIRCompilationResult,
                                                 basic_block: &BasicBlock,
                                                 mut register_is_null: HashMap<VirtualRegister, bool>) -> InstructionsRegisterNullStatus {
    let mut instructions_status = Vec::new();

    for instruction_index in &basic_block.instructions {
        let instruction = &compilation_result.instructions[*instruction_index];

        instructions_status.push(register_is_null.clone());

        match &instruction.data {
            InstructionMIRData::LoadInt32(_, _) => {}
            InstructionMIRData::LoadFloat32(_, _) => {}
            InstructionMIRData::Move(destination, source) => {
                if source.value_type.is_reference() && destination.value_type.is_reference() {
                    register_is_null.insert(destination.clone(), register_is_null[source]);
                }
            }
            InstructionMIRData::AddInt32(_, _, _) => {}
            InstructionMIRData::SubInt32(_, _, _) => {}
            InstructionMIRData::AddFloat32(_, _, _) => {}
            InstructionMIRData::SubFloat32(_, _, _) => {}
            InstructionMIRData::Return(_) => {}
            InstructionMIRData::Call(_, destination, _) => {
                if let Some(destination) = destination {
                    if destination.value_type.is_reference() {
                        register_is_null.insert(destination.clone(), true);
                    }
                }
            }
            InstructionMIRData::LoadArgument(index, destination) => {
                if function.definition().parameters()[*index as usize].is_reference() {
                    register_is_null.insert(destination.clone(), true);
                }
            }
            InstructionMIRData::LoadNull(destination) => {
                register_is_null.insert(destination.clone(), true);
            }
            InstructionMIRData::NewArray(_, destination, _) => {
                register_is_null.insert(destination.clone(), false);
            }
            InstructionMIRData::LoadElement(_, destination, _, _) => {
                if destination.value_type.is_reference() {
                    register_is_null.insert(destination.clone(), true);
                }
            }
            InstructionMIRData::StoreElement(_, _, _, _) => {}
            InstructionMIRData::LoadArrayLength(_, _) => {}
            InstructionMIRData::BranchLabel(_) => {}
            InstructionMIRData::Branch(_) => {}
            InstructionMIRData::BranchCondition(_, _, _, _, _) => {}
        }
    }

    instructions_status
}

fn merge_results(mut potential_results: Vec<InstructionsRegisterNullStatus>) -> InstructionsRegisterNullStatus  {
    let mut final_result = potential_results.remove(0);

    for potential_result in potential_results {
        for (instruction_index, instruction) in potential_result.iter().enumerate() {
            for (register, &is_null) in instruction {
                let current_is_null = final_result[instruction_index].get(register).cloned().unwrap_or(false);
                final_result[instruction_index].insert(
                    register.clone(),
                    is_null || current_is_null
                );
            }
        }
    }

    final_result
}

#[test]
fn test_no_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Array(Box::new(Type::Int32))),
        vec![],
        vec![
            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[1].len());
    assert_eq!(true, result[1][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches2() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Array(Box::new(Type::Int32))),
        vec![],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[2].len());
    assert_eq!(false, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches3() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[2].len());
    assert_eq!(false, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(1, result[3].len());
    assert_eq!(false, result[3][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(1, result[4].len());
    assert_eq!(false, result[4][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches4() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[0].len());
    assert_eq!(true, result[0][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[1].len());
    assert_eq!(true, result[1][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[1][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[2].len());
    assert_eq!(true, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[2][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[3].len());
    assert_eq!(true, result[3][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[3][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches5() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[0].len());
    assert_eq!(true, result[0][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(1, result[1].len());
    assert_eq!(true, result[1][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[2].len());
    assert_eq!(true, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[2][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[3].len());
    assert_eq!(false, result[3][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[3][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[4].len());
    assert_eq!(false, result[4][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[4][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[5].len());
    assert_eq!(false, result[5][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[5][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[6].len());
    assert_eq!(false, result[6][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[6][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches6() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Int32),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Int32),

            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::StoreLocal(0),

            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[0].len());
    assert_eq!(true, result[0][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(1, result[1].len());
    assert_eq!(true, result[1][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[2].len());
    assert_eq!(true, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[2][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[3].len());
    assert_eq!(false, result[3][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[3][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[4].len());
    assert_eq!(false, result[4][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[4][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[5].len());
    assert_eq!(false, result[5][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[5][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(2, result[6].len());
    assert_eq!(false, result[6][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[6][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(3, result[7].len());
    assert_eq!(false, result[7][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[7][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[7][&VirtualRegister::new(2, Type::Array(Box::new(Type::Int32)))]);

    assert_eq!(3, result[8].len());
    assert_eq!(true, result[8][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(false, result[8][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[8][&VirtualRegister::new(2, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_no_branches7() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Array(Box::new(Type::Int32))),
        vec![],
        vec![
            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Array(Box::new(Type::Int32))),
            Instruction::LoadInt32(0),
            Instruction::LoadElement(Type::Array(Box::new(Type::Int32))),
            Instruction::Return,
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);

    let result = compute_null_check_elision_for_block(&function, &compilation_result, &basic_blocks[0]);

    for instruction in &result {
        println!("{:?}", instruction);
    }

    assert_eq!(1, result[2].len());
    assert_eq!(false, result[2][&VirtualRegister::new(0, Type::Array(Box::new(Type::Array(Box::new(Type::Int32)))))]);

    assert_eq!(1, result[3].len());
    assert_eq!(false, result[3][&VirtualRegister::new(0, Type::Array(Box::new(Type::Array(Box::new(Type::Int32)))))]);

    assert_eq!(2, result[4].len());
    assert_eq!(false, result[4][&VirtualRegister::new(0, Type::Array(Box::new(Type::Array(Box::new(Type::Int32)))))]);
    assert_eq!(true, result[4][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
}

#[test]
fn test_branches1() {
    let mut function = Function::new(
        FunctionDefinition::new_managed("test".to_owned(), vec![], Type::Array(Box::new(Type::Int32))),
        vec![Type::Array(Box::new(Type::Int32))],
        vec![
            Instruction::LoadInt32(0),
            Instruction::LoadInt32(0),
            Instruction::BranchEqual(6),

            Instruction::LoadNull(Type::Array(Box::new(Type::Int32))),
            Instruction::StoreLocal(0),
            Instruction::Branch(9),

            Instruction::LoadInt32(1000),
            Instruction::NewArray(Type::Int32),
            Instruction::StoreLocal(0),

            Instruction::LoadLocal(0),
            Instruction::Return
        ]
    );

    let binder = Binder::new();
    Verifier::new(&binder, &mut function).verify().unwrap();

    let mut compiler = InstructionMIRCompiler::new(&binder, &function);
    compiler.compile(function.instructions());
    let compilation_result = compiler.done();
    let basic_blocks = BasicBlock::create_blocks(&compilation_result.instructions);
    let control_flow_graph = ControlFlowGraph::new(
        &compilation_result.instructions,
        &basic_blocks,
        &branches::create_label_mapping(&compilation_result.instructions)
    );

    let result = compute_null_check_elision(&function, &compilation_result, &basic_blocks, &control_flow_graph);

    for (block_index, block) in basic_blocks.iter().enumerate() {
        println!("Block #{}", block_index);
        for &instruction_index in &block.instructions {
            println!("\t{}: {:?}", compilation_result.instructions[instruction_index].name(), result[instruction_index]);
        }
    }

    assert_eq!(2, result[12].len());
    assert_eq!(true, result[12][&VirtualRegister::new(0, Type::Array(Box::new(Type::Int32)))]);
    assert_eq!(true, result[12][&VirtualRegister::new(1, Type::Array(Box::new(Type::Int32)))]);
}