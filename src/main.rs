#![allow(unused_imports)]

#[macro_use]
extern crate lazy_static;

mod model;
mod mir;
mod analysis;
mod compiler;
mod optimization;
mod runtime;
mod engine;
mod vm;
mod parser;
mod execution_tests;
mod integration_tests;

use crate::vm::VirtualMachine;
use crate::parser::Parser;
use crate::engine::execution::{ExecutionEngineResult, ExecutionEngineError};

pub fn main_execute(input_file: String) -> ExecutionEngineResult<i32> {
    let mut vm = VirtualMachine::new();

    let input_text = std::fs::read_to_string(input_file).map_err(|err| ExecutionEngineError::Other(format!("{}", err)))?;
    let tokens = parser::tokenize(&input_text).map_err(|err| ExecutionEngineError::Other(format!("{:?}", err)))?;

    let mut parser = Parser::new(tokens);
    let (functions, classes)  = parser.parse().map_err(|err| ExecutionEngineError::Other(format!("{:?}", err)))?;

    for function in functions {
        vm.add_function(function)?;
    }

    for class in classes {
        vm.add_class(class);
    }

    vm.execute()
}

fn main() {
    let input_file = std::env::args().collect::<Vec<_>>().get(1).expect("Expected input file.").clone();
    let result = main_execute(input_file).unwrap();
    println!("{}", result);
}
