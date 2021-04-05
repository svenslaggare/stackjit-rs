use std::str::FromStr;

use crate::model::function::{Function, FunctionDeclaration, FunctionSignature};
use crate::model::typesystem::TypeId;
use crate::model::instruction::Instruction;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    LeftCurlyParentheses,
    RightCurlyParentheses,
    LeftParentheses,
    RightParentheses,
    Int32(i32),
    Float32(f32),
    Identifier(String),
    DefineNumberOfLocals,
    DefineLocal,
    Function,
    Class,
    Colon,
    End
}

#[derive(Debug)]
pub enum ParserError {
    FloatConvertError,
    IntConvertError,
    AlreadyHasDot,
    ReachedEndOfTokens,
    ExpectedFunctionOrClass,
    ExpectedIdentifier,
    ExpectedInt32,
    ExpectedFloat32,
    ExpectedLeftParentheses,
    ExpectedRightParentheses,
    ExpectedLeftCurlyParentheses,
    ExpectedRightCurlyParentheses,
    NotDefinedType(String),
    NotDefinedInstruction(String),
    UndefinedModifier,
    UntypedLocal(u32),
    ExpectedColon
}

pub type ParserResult<T> = Result<T, ParserError>;

pub fn tokenize(text: &str) -> ParserResult<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut char_iterator = text.chars().peekable();

    while let Some(current) = char_iterator.next() {
        if current.is_alphabetic() {
            let mut identifier = String::new();
            identifier.push(current);

            loop {
                match char_iterator.peek() {
                    Some(next) if next.is_alphanumeric() || next == &'_' => {
                        identifier.push(char_iterator.next().unwrap());
                    }
                    _ => {
                        break
                    }
                };
            }

            if identifier == "func" {
                tokens.push(Token::Function);
            } else if identifier == "class" {
                tokens.push(Token::Class);
            } else {
                tokens.push(Token::Identifier(identifier));
            }
        } else if current.is_numeric() {
            let mut number = String::new();
            number.push(current);
            let mut has_dot = false;

            loop {
                match char_iterator.peek() {
                    Some(next) if next.is_numeric() => {
                        number.push(char_iterator.next().unwrap());
                    }
                    Some(next) if next == &'.' => {
                        if has_dot {
                            return Err(ParserError::AlreadyHasDot);
                        }

                        has_dot = true;
                        number.push(char_iterator.next().unwrap());
                    }
                    _ => {
                        break
                    }
                };
            }

            if has_dot {
                tokens.push(Token::Float32(f32::from_str(&number).map_err(|_err| ParserError::FloatConvertError)?));
            } else {
                tokens.push(Token::Int32(i32::from_str(&number).map_err(|_err| ParserError::IntConvertError)?));
            }
        } else if current == '.' {
            let mut identifier = String::new();
            identifier.push(current);

            loop {
                match char_iterator.peek() {
                    Some(next) if next.is_alphanumeric() || next == &'_' => {
                        identifier.push(char_iterator.next().unwrap());
                    }
                    _ => {
                        break
                    }
                };
            }

            if identifier == ".locals" {
                tokens.push(Token::DefineNumberOfLocals);
            } else if identifier == ".local" {
                tokens.push(Token::DefineLocal);
            } else {
                return Err(ParserError::UndefinedModifier);
            }
        } else if current == '(' {
            tokens.push(Token::LeftParentheses);
        } else if current == ')' {
            tokens.push(Token::RightParentheses);
        } else if current == '{' {
            tokens.push(Token::LeftCurlyParentheses);
        } else if current == '}' {
            tokens.push(Token::RightCurlyParentheses);
        } else if current == ':' {
            tokens.push(Token::Colon);
        } else if current.is_whitespace() {
            // Skip
        }
    }

    tokens.push(Token::End);

    Ok(tokens)
}

pub struct Parser {
    tokens: Vec<Token>,
    index: isize,
    functions: Vec<Function>
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            index: -1,
            functions: Vec::new()
        }
    }

    pub fn parse(&mut self) -> ParserResult<Vec<Function>> {
        loop {
            self.next()?;
            self.parse_top_level()?;

            if self.current() == &Token::End {
                break;
            }
        }

        Ok(std::mem::take(&mut self.functions))
    }

    fn parse_top_level(&mut self) -> ParserResult<()> {
        match self.current().clone() {
            Token::Function => {
                let function = self.parse_function()?;
                self.functions.push(function);
                Ok(())
            }
            Token::Class => {
                Ok(())
            }
            _ => { return Err(ParserError::ExpectedFunctionOrClass); }
        }
    }

    fn parse_function(&mut self) -> ParserResult<Function> {
        self.next()?;
        let name = self.next_identifier()?;

        match self.current() {
            Token::LeftParentheses => { self.next()?; }
            _ => { return Err(ParserError::ExpectedLeftParentheses); }
        }

        let mut parameters = Vec::new();
        loop {
            match self.current() {
                Token::RightParentheses => {
                    self.next()?;
                    break;
                }
                Token::Identifier(identifier) => {
                    parameters.push(parse_type(identifier)?);
                    self.next()?;
                }
                _ => { return Err(ParserError::ExpectedRightParentheses); }
            }
        }

        let return_type = self.next_type_id()?;

        match self.current() {
            Token::LeftCurlyParentheses => { self.next()?; }
            _ => { return Err(ParserError::ExpectedLeftCurlyParentheses); }
        }

        let mut instructions = Vec::new();
        let mut locals = Vec::new();

        loop {
            let current = self.current().clone();
            match current {
                Token::RightCurlyParentheses => {
                    self.next()?;
                    break;
                }
                Token::DefineNumberOfLocals => {
                    self.next()?;

                    let num_locals = self.next_i32()? as usize;
                    locals.resize(num_locals, None);
                }
                Token::DefineLocal => {
                    self.next()?;

                    let index = self.next_i32()? as usize;
                    let local_type = self.next_type_id()?;
                    locals[index] = Some(local_type);
                }
                Token::Identifier(instruction_name) => {
                    instructions.push(self.parse_instruction(&instruction_name)?);
                }
                _ => { return Err(ParserError::ExpectedIdentifier); }
            }
        }

        let mut locals_checked = Vec::new();
        for (index, local) in locals.into_iter().enumerate() {
            if let Some(local) = local {
                locals_checked.push(local);
            } else {
                return Err(ParserError::UntypedLocal(index as u32));
            }
        }

        Ok(Function::new(
            FunctionDeclaration::new_managed(
                name,
                parameters,
                return_type
            ),
            locals_checked,
            instructions
        ))
    }

    fn parse_instruction(&mut self, identifier: &str) -> ParserResult<Instruction> {
        self.next()?;

        match identifier.to_lowercase().as_str() {
            "ldnull" => {
                let reference_type = self.next_type_id()?;
                Ok(Instruction::LoadNull(reference_type))
            }
            "ldint" => {
                let value = self.next_i32()?;
                Ok(Instruction::LoadInt32(value))
            }
            "ldfloat" => {
                let value = self.next_f32()?;
                Ok(Instruction::LoadFloat32(value))
            }
            "ldloc" => {
                let value = self.next_i32()?;
                Ok(Instruction::LoadLocal(value as u32))
            }
            "stloc" => {
                let value = self.next_i32()?;
                Ok(Instruction::StoreLocal(value as u32))
            }
            "newarr" => {
                let element_type = self.next_type_id()?;
                Ok(Instruction::NewArray(element_type))
            }
            "ldelem" => {
                let element_type = self.next_type_id()?;
                Ok(Instruction::LoadElement(element_type))
            }
            "stelem" => {
                let element_type = self.next_type_id()?;
                Ok(Instruction::StoreElement(element_type))
            }
            "ldlen" => { Ok(Instruction::LoadArrayLength) }
            "add" => { Ok(Instruction::Add) }
            "sub" => { Ok(Instruction::Sub) }
            "ldarg" => {
                let argument = self.next_i32()?;
                Ok(Instruction::LoadArgument(argument as u32))
            }
            "call" => {
                let call_name = self.next_identifier()?;
                let mut arguments = Vec::new();

                match self.current() {
                    Token::LeftParentheses => {
                        self.next()?;
                    }
                    _ => { return Err(ParserError::ExpectedLeftParentheses); }
                }

                loop {
                    match self.current() {
                        Token::Identifier(identifier) => {
                            let argument = parse_type(identifier)?;
                            arguments.push(argument);
                            self.next()?;
                        }
                        Token::RightParentheses => {
                            self.next()?;
                            break;
                        }
                        _ => {
                            return Err(ParserError::ExpectedIdentifier);
                        }
                    }
                }

                Ok(Instruction::Call(FunctionSignature::new(call_name, arguments)))
            }
            "ret" => { Ok(Instruction::Return) }
            "newobj" => {
                let class_type = self.next_identifier()?;
                Ok(Instruction::NewObject(class_type))
            }
            "ldfield" => {
                let class_name = self.next_identifier()?;

                match self.current() {
                    Token::Colon => { self.next()?; }
                    _ => { return Err(ParserError::ExpectedColon); }
                }

                match self.current() {
                    Token::Colon => { self.next()?; }
                    _ => { return Err(ParserError::ExpectedColon); }
                }

                let field_name = self.next_identifier()?;
                Ok(Instruction::LoadField(class_name, field_name))
            }
            "stfield" => {
                let class_name = self.next_identifier()?;

                match self.current() {
                    Token::Colon => { self.next()?; }
                    _ => { return Err(ParserError::ExpectedColon); }
                }

                match self.current() {
                    Token::Colon => { self.next()?; }
                    _ => { return Err(ParserError::ExpectedColon); }
                }

                let field_name = self.next_identifier()?;
                Ok(Instruction::StoreField(class_name, field_name))
            }
            "br" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::Branch(target))
            }
            "beq" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchEqual(target))
            }
            "bne" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchNotEqual(target))
            }
            "bgt" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchGreaterThan(target))
            }
            "bge" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchGreaterThanOrEqual(target))
            }
            "blt" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchLessThan(target))
            }
            "ble" => {
                let target = self.next_i32()? as u32;
                Ok(Instruction::BranchLessThanOrEqual(target))
            }
            _ => { return Err(ParserError::NotDefinedInstruction(identifier.to_owned())); }
        }
    }

    fn next_type_id(&mut self) -> ParserResult<TypeId> {
        parse_type(&self.next_identifier()?)
    }

    fn next_identifier(&mut self) -> ParserResult<String> {
        match self.current().clone() {
            Token::Identifier(identifier) => {
                self.next()?;
                Ok(identifier.clone())
            }
            _ => { return Err(ParserError::ExpectedIdentifier); }
        }
    }

    fn next_i32(&mut self) -> ParserResult<i32> {
        match self.current().clone() {
            Token::Int32(value) => {
                self.next()?;
                Ok(value)
            }
            _ => { return Err(ParserError::ExpectedInt32); }
        }
    }

    fn next_f32(&mut self) -> ParserResult<f32> {
        match self.current().clone() {
            Token::Float32(value) => {
                self.next()?;
                Ok(value)
            }
            _ => { return Err(ParserError::ExpectedFloat32); }
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index as usize]
    }

    fn next(&mut self) -> ParserResult<&Token> {
        self.index += 1;
        if self.index >= self.tokens.len() as isize {
            return Err(ParserError::ReachedEndOfTokens);
        }

        Ok(&self.tokens[self.index as usize])
    }
}

fn parse_type(type_str: &str) -> ParserResult<TypeId> {
    TypeId::from_str(type_str).ok_or_else(|| ParserError::NotDefinedType(type_str.to_owned()))
}

#[test]
fn test_parse_function1() {
    let text = r"
    func test(Int Int) Int
    {
        LDINT 100
        LDINT 200
        ADD
        RET
    }
    ";

    let mut parser = Parser::new(tokenize(text).unwrap());
    let functions = parser.parse().unwrap();

    assert_eq!(1, functions.len());

    let function = &functions[0];
    assert_eq!("test", function.declaration().name());
    assert_eq!(&vec![TypeId::Int32, TypeId::Int32], function.declaration().parameters());
    assert_eq!(&TypeId::Int32, function.declaration().return_type());

    assert_eq!(Instruction::LoadInt32(100), function.instructions()[0]);
    assert_eq!(Instruction::LoadInt32(200), function.instructions()[1]);
    assert_eq!(Instruction::Add, function.instructions()[2]);
    assert_eq!(Instruction::Return, function.instructions()[3]);
}


#[test]
fn test_parse_function2() {
    let text = r"
    func test(Int Int) Int
    {
        .locals 1
        .local 0 Int
        LDINT 100
        LDLOC 0
        ADD
        RET
    }
    ";

    let mut parser = Parser::new(tokenize(text).unwrap());
    let functions = parser.parse().unwrap();

    assert_eq!(1, functions.len());

    let function = &functions[0];
    assert_eq!("test", function.declaration().name());
    assert_eq!(&vec![TypeId::Int32, TypeId::Int32], function.declaration().parameters());
    assert_eq!(&TypeId::Int32, function.declaration().return_type());
    assert_eq!(&vec![TypeId::Int32], function.locals());

    assert_eq!(Instruction::LoadInt32(100), function.instructions()[0]);
    assert_eq!(Instruction::LoadLocal(0), function.instructions()[1]);
    assert_eq!(Instruction::Add, function.instructions()[2]);
    assert_eq!(Instruction::Return, function.instructions()[3]);
}

#[test]
fn test_parse_function3() {
    let text = r"
    func test(Int Int) Int
    {
        .locals 1
        .local 0 Int
        LDINT 100
        LDLOC 0
        CALL addInt(Int Int)
        RET
    }
    ";

    let mut parser = Parser::new(tokenize(text).unwrap());
    let functions = parser.parse().unwrap();

    assert_eq!(1, functions.len());

    let function = &functions[0];
    assert_eq!("test", function.declaration().name());
    assert_eq!(&vec![TypeId::Int32, TypeId::Int32], function.declaration().parameters());
    assert_eq!(&TypeId::Int32, function.declaration().return_type());
    assert_eq!(&vec![TypeId::Int32], function.locals());

    assert_eq!(Instruction::LoadInt32(100), function.instructions()[0]);
    assert_eq!(Instruction::LoadLocal(0), function.instructions()[1]);
    assert_eq!(Instruction::Call(FunctionSignature { name: "addInt".to_string(), parameters: vec![TypeId::Int32, TypeId::Int32] }), function.instructions()[2]);
    assert_eq!(Instruction::Return, function.instructions()[3]);
}

#[test]
fn test_parse_function4() {
    let text = r"
    func test() Int
    {
        NEWOBJ Point
        LDFIELD Point::x
        RET
    }
    ";

    let mut parser = Parser::new(tokenize(text).unwrap());
    let functions = parser.parse().unwrap();

    assert_eq!(1, functions.len());

    let function = &functions[0];
    assert_eq!("test", function.declaration().name());
    assert_eq!(&Vec::<TypeId>::new(), function.declaration().parameters());
    assert_eq!(&TypeId::Int32, function.declaration().return_type());

    assert_eq!(Instruction::NewObject("Point".to_owned()), function.instructions()[0]);
    assert_eq!(Instruction::LoadField("Point".to_owned(), "x".to_owned()), function.instructions()[1]);
    assert_eq!(Instruction::Return, function.instructions()[2]);
}

#[test]
fn test_parse_function5() {
    let text = r"
    func test() Int
    {
        LDINT 1
        LDINT 2
        BNE 6
        LDINT 1337
        STLOC 0
        BR 8
        LDINT 4711
        STLOC 0
        LDLOC 0
        RET
    }
    ";

    let mut parser = Parser::new(tokenize(text).unwrap());
    let functions = parser.parse().unwrap();

    assert_eq!(1, functions.len());

    let function = &functions[0];
    assert_eq!("test", function.declaration().name());
    assert_eq!(&Vec::<TypeId>::new(), function.declaration().parameters());
    assert_eq!(&TypeId::Int32, function.declaration().return_type());

    assert_eq!(Instruction::LoadInt32(1), function.instructions()[0]);
    assert_eq!(Instruction::LoadInt32(2), function.instructions()[1]);
    assert_eq!(Instruction::BranchNotEqual(6), function.instructions()[2]);
    assert_eq!(Instruction::LoadInt32(1337), function.instructions()[3]);
    assert_eq!(Instruction::StoreLocal(0), function.instructions()[4]);
    assert_eq!(Instruction::Branch(8), function.instructions()[5]);
    assert_eq!(Instruction::LoadInt32(4711), function.instructions()[6]);
    assert_eq!(Instruction::StoreLocal(0), function.instructions()[7]);
    assert_eq!(Instruction::LoadLocal(0), function.instructions()[8]);
    assert_eq!(Instruction::Return, function.instructions()[9]);
}