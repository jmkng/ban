use crate::{
    compile::{Keyword, Operator},
    Marker,
};
use std::fmt::Display;

/// Types emitted by the Lexer.
///
/// An abstraction over raw text to make construction of Tree types easier.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token {
    /// Raw text.
    Raw,
    /// String literal within a tag.
    String,
    /// Number within a tag.
    Number,
    /// Identifier (unquoted string) within a tag.
    Identifier,
    /// Whitespace within a tag.
    Whitespace,
    /// Beginning of an expression - (( by default.
    BeginExpression,
    /// End of an expression - )) by default.
    EndExpression,
    /// Beginning of a block - (* by default.
    BeginBlock,
    /// End of a block - *) by default.
    EndBlock,
    /// .
    Period,
    /// ||
    Or,
    /// &&
    And,
    /// |
    Pipe,
    /// =
    Assign,
    /// A boolean true.
    True,
    /// A boolean false.
    False,
    /// !
    Exclamation,
    /// :
    Colon,
    /// A recognized "special" keyword that begins a certain type of block.
    Keyword(Keyword),
    /// Describes an action taken on two values.
    Operator(Operator),
}

impl Token {
    /// Convert a Marker into a Token.
    ///
    /// Return value includes the resulting Token and a boolean which indicates
    /// if the Token is whitespace trimmed.
    pub(crate) fn from_usize_trim(id: usize) -> (Self, bool) {
        match Marker::from(id) {
            Marker::BeginExpression => (Self::BeginExpression, false),
            Marker::EndExpression => (Self::EndExpression, false),
            Marker::BeginExpressionTrim => (Self::BeginExpression, true),
            Marker::EndExpressionTrim => (Self::EndExpression, true),
            Marker::BeginBlock => (Self::BeginBlock, false),
            Marker::EndBlock => (Self::EndBlock, false),
            Marker::BeginBlockTrim => (Self::BeginBlock, true),
            Marker::EndBlockTrim => (Self::EndBlock, true),
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Raw => write!(f, "raw"),
            Token::String => write!(f, "string"),
            Token::Number => write!(f, "number"),
            Token::Identifier => write!(f, "identifer"),
            Token::Whitespace => write!(f, "whitespace"),
            Token::BeginExpression => write!(f, "begin expression"),
            Token::EndExpression => write!(f, "end expression"),
            Token::BeginBlock => write!(f, "begin block"),
            Token::EndBlock => write!(f, "end block"),
            Token::Period => write!(f, "period (.)"),
            Token::Assign => write!(f, "assign (=)"),
            Token::Keyword(keyword) => write!(f, "keyword {keyword}"),
            Token::Operator(operator) => write!(f, "operator {operator}"),
            Token::Pipe => write!(f, "pipe (|)"),
            Token::Exclamation => write!(f, "exclamation (!)"),
            Token::Colon => write!(f, "colon (:)"),
            Token::Or => write!(f, "or (||)"),
            Token::And => write!(f, "and (&&)"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
        }
    }
}
