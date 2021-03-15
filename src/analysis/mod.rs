use crate::analysis::null_check_elision::InstructionsRegisterNullStatus;

pub mod basic_block;
pub mod control_flow_graph;
pub mod liveness;
pub mod null_check_elision;

pub struct AnalysisResult {
    pub instructions_register_null_status: InstructionsRegisterNullStatus
}