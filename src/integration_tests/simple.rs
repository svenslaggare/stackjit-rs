use crate::main_execute;

#[test]
fn test_simple1() {
    let result = main_execute("test_programs/simple/simple1.sbc".to_owned());
    assert_eq!(Ok(300), result);
}