use crate::analysis::liveness::LiveInterval;
use crate::compiler::ir::HardwareRegister;

pub mod register_allocation;
pub mod null_check_elision;
pub mod peephole;
