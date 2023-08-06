use crate::compile::lex::Token;

/// Describes the internal state of a [`Lexer`][`super::Lexer`].
#[derive(Debug, PartialEq)]
pub enum CursorState {
    /// Indicates the [`Lexer`][`super::Lexer`] is not inside of a block
    /// or expression.
    Default,
    /// Indicates the [`Lexer`][`super::Lexer`] is inside of a block or
    /// expression.
    Inside {
        /// The expected ending [`Token`].
        end_token: Token,
    },
}
