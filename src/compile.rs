mod lex;
mod parse;
mod template;

pub use crate::compile::{
    lex::token,
    parse::{scope::Scope, tree, Parser},
    template::Template,
};

use crate::log::Error;
use std::fmt::Display;

/// Compile a [`Template`] from the given text.
///
/// Provides a shortcut to quickly compile a `Template` without creating
/// an `Engine`.
///
/// # Examples
///
/// ```
/// use ban::compile;
///
/// let template = compile("(( name ))");
/// assert!(template.is_ok())
/// ```
pub fn compile<'source>(text: &'source str) -> Result<Template, Error> {
    Parser::new(text).compile(None)
}

/// Keywords recognized by the Lexer and Parser.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Keyword {
    /// Enables negation.
    Not,
    /// Beginning of an "if" block.
    If,
    /// Marks the beginning of the else_branch in an "if" block.
    Else,
    /// Beginning of an assignment.
    Let,
    /// Beginning of a loop.
    For,
    /// Divides the identifier from the keys in a loop.
    ///
    /// In this example, identifier refers to "person" while keys
    /// refers to "people":
    ///
    /// "for person in people"
    In,
    /// Beginning of an include block.
    Include,
    /// Beginning of an extends expression.
    Extends,
    /// Beginning of a "block" block.
    Block,
    /// End of a block.
    End,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::Not => write!(f, "not"),
            Keyword::If => write!(f, "if"),
            Keyword::Else => write!(f, "else"),
            Keyword::Let => write!(f, "let"),
            Keyword::For => write!(f, "for"),
            Keyword::In => write!(f, "in"),
            Keyword::Include => write!(f, "include"),
            Keyword::Extends => write!(f, "extends'"),
            Keyword::Block => write!(f, "block"),
            Keyword::End => write!(f, "end"),
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
    /// >
    Greater,
    /// <
    Lesser,
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
            Operator::Add => write!(f, "+"),
            Operator::Subtract => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::Greater => write!(f, ">"),
            Operator::Lesser => write!(f, "<"),
            Operator::Equal => write!(f, "=="),
            Operator::NotEqual => write!(f, "!="),
            Operator::GreaterOrEqual => write!(f, ">="),
            Operator::LesserOrEqual => write!(f, "<="),
        }
    }
}
