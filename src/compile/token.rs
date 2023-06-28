use crate::Marker;

#[derive(Debug, PartialEq)]
pub enum Token {
    /// Raw text.
    Raw,
    /// String literal within a tag.
    String,
    /// Number within a tag.
    Number,
    /// Identifier (unquoted string) within a tag.
    Ident,
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
    /// A recognized "special" keyword that begins a certain type of block.
    Keyword(Keyword),
    /// A dummy no-op token type.
    Dummy,
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

impl Default for Token {
    fn default() -> Self {
        Token::Dummy
    }
}

#[derive(Debug, PartialEq)]
pub enum Keyword {
    If,
    Let,
    For,
    Include,
}
