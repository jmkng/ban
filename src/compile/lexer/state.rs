use super::Token;

/// Describes the possible Lexer states.
#[derive(Debug, PartialEq)]
pub(crate) enum State {
    /// Indicates the lexer is not inside of a block or expression.
    Default,
    /// Tag refers to a Block or Expression.
    Tag {
        /// The expected ending, either "))" or "*)" by default.
        end_token: Token,
    },
}
