mod lexer;
pub(crate) mod parser;
mod template;

pub use crate::compile::parser::Tree;
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
    Else,
    True,
    False,
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
            Keyword::Else => write!(f, "else"),
            Keyword::True => write!(f, "true"),
            Keyword::False => write!(f, "false"),
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
    /// ||
    Or,
    /// &&
    And,
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
            Operator::Or => write!(f, "or (||)"),
            Operator::And => write!(f, "and (&&)"),
        }
    }
}
