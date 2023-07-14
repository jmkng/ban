mod lexer;
mod parser;
mod template;

pub use crate::compile::{
    parser::{scope::Scope, tree, Parser},
    template::Template,
};

use crate::Error;
use std::fmt::Display;

/// Compile a template.
///
/// Provides a shortcut to quickly compile a Template without creating
/// an Engine.
///
/// If you create a Template that relies on custom filter functions,
/// you will need to use [ash::new()] to create an Engine instance
/// which can store your filters.
///
/// This Engine should also be used to perform the render.
pub fn compile<'source>(text: &'source str) -> Result<Template, Error> {
    Parser::new(text).compile()
}

/// Keywords recognized by the Lexer and Parser.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Keyword {
    /// Beginning of an if expression.
    If,
    /// Marks the beginning of the else_branch in an if expression.
    Else,
    /// End of an if expression.
    EndIf,
    /// Beginning of an assignment.
    Let,
    /// Beginning of a loop.
    For,
    /// Divides the identifier from the context in a loop.
    ///
    /// In this example, identifier refers to "person" while context
    /// refers to "people":
    ///
    /// "for person in people"
    In,
    /// End of a loop.
    EndFor,
    /// Beginning of an include expression.
    Include,
    /// Beginning of an extends expression.
    Extends,
    /// Beginning of a block expression.
    Block,
    /// Ending of a block expression.
    EndBlock,
    /// A boolean true.
    True,
    /// A boolean false.
    False,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::If => write!(f, "if"),
            Keyword::Else => write!(f, "else"),
            Keyword::EndIf => write!(f, "endif"),
            Keyword::Let => write!(f, "let"),
            Keyword::For => write!(f, "for"),
            Keyword::In => write!(f, "in"),
            Keyword::EndFor => write!(f, "endfor"),
            Keyword::Include => write!(f, "include"),
            Keyword::Extends => write!(f, "extends'"),
            Keyword::Block => write!(f, "block"),
            Keyword::EndBlock => write!(f, "endblock"),
            Keyword::True => write!(f, "true"),
            Keyword::False => write!(f, "false"),
        }
    }
}

/// Operators recognized by the Lexer and Parser.
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
