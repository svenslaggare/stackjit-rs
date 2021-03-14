use crate::vm::VirtualMachine;
use crate::model::function::{Function, FunctionDefinition};
use crate::model::instruction::Instruction;
use crate::model::typesystem::Type;

#[test]
fn test_sum1() {
    let mut vm = VirtualMachine::new();

    let n = 20000000;

    vm.engine.add_function(Function::new(
        FunctionDefinition::new_managed("main".to_owned(), Vec::new(), Type::Int32),
        vec![Type::Int32],
        vec![
            Instruction::LoadLocal(0),
            Instruction::LoadInt32(1),
            Instruction::Add,
            Instruction::StoreLocal(0),

            Instruction::LoadInt32(n),
            Instruction::LoadLocal(0),
            Instruction::BranchGreaterThan(0),

            Instruction::LoadLocal(0),
            Instruction::Return,
        ]
    )).unwrap();

    let execution_result = vm.execute().unwrap();
    assert_eq!(n, execution_result);
}