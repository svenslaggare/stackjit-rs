use std::collections::{HashMap, HashSet};

use crate::ir::low::BranchLabel;
use crate::ir::mid::InstructionMIR;
use crate::model::instruction;
use crate::model::instruction::Instruction;

pub struct BranchManager {
    branch_targets: HashSet<instruction::BranchTarget>,
    branch_labels: HashMap<instruction::BranchTarget, BranchLabel>,
    next_branch_label: BranchLabel
}

impl BranchManager {
    pub fn new() -> BranchManager {
        BranchManager {
            branch_targets: HashSet::new(),
            branch_labels: HashMap::new(),
            next_branch_label: 0
        }
    }

    pub fn define_branch_labels(&mut self, instructions: &Vec<Instruction>) {
        for instruction in instructions {
            if let Some(target) = instruction.branch_target() {
                self.branch_targets.insert(target);

                if !self.branch_labels.contains_key(&target) {
                    let label = self.next_branch_label;
                    self.next_branch_label += 1;
                    self.branch_labels.insert(target, label);
                }
            }
        }
    }

    pub fn is_branch(&self, instruction_index: usize) -> Option<BranchLabel> {
        let branch_target = instruction_index as instruction::BranchTarget;
        if self.branch_targets.contains(&branch_target) {
            Some(self.branch_labels[&branch_target])
        } else {
            None
        }
    }

    pub fn get_label(&self, target: instruction::BranchTarget) -> Option<BranchLabel> {
        self.branch_labels.get(&target).cloned()
    }
}

pub fn create_label_mapping(instructions: &Vec<InstructionMIR>) -> HashMap<BranchLabel, usize> {
    let mut mapping = HashMap::new();

    for (instruction_index, instruction) in instructions.iter().enumerate() {
        if let InstructionMIR::BranchLabel(label) = instruction {
            mapping.insert(*label, instruction_index);
        }
    }

    mapping
}