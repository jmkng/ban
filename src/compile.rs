mod lexer;
mod parser;
mod template;

use crate::{
    compile::{parser::Parser, template::Template},
    error::Error,
};
use std::fmt::Display;

pub fn compile<'source>(text: &'source str) -> Result<Template, Error> {
    Parser::new(text).compile()
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Keyword {
    If,
    Let,
    For,
    In,
    Include,
    EndFor,
    EndIf,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::If => write!(f, "if"),
            Keyword::Let => write!(f, "let"),
            Keyword::For => write!(f, "for"),
            Keyword::Include => write!(f, "include"),
            Keyword::In => write!(f, "in"),
            Keyword::EndFor => write!(f, "endfor"),
            Keyword::EndIf => write!(f, "endif"),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Operator {
    /// +
    Add,
    /// -
    Subtract,
    /// *
    Multiply,
    /// /
    Divide,
    /// =
    Assign,
    /// ==
    Equal,
    /// !=
    NotEqual,
    /// >=
    GreaterOrEqual,
    /// <=
    LesserOrEqual,
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Add => write!(f, "add (+)"),
            Operator::Subtract => write!(f, "subtract (-)"),
            Operator::Multiply => write!(f, "multiply (*)"),
            Operator::Divide => write!(f, "divide (/)"),
            Operator::Assign => write!(f, "assign (=)"),
            Operator::Equal => write!(f, "equal (==)"),
            Operator::NotEqual => write!(f, "not equal (!=)"),
            Operator::GreaterOrEqual => write!(f, "greater or equal (>=)"),
            Operator::LesserOrEqual => write!(f, "lesser or equal (<=)"),
        }
    }
}
