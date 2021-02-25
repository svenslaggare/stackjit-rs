#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Int32,
    Float32
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => {
                write!(f, "Void")
            }
            Type::Int32 => {
                write!(f, "Int32")
            }
            Type::Float32 => {
                write!(f, "Float32")
            }
        }
    }
}