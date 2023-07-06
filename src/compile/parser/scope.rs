use crate::compile::lexer::Token;

pub struct Scope {
    pub statements: Vec<Token>,
}

impl Scope {
    /// Create a new Scope.
    #[inline]
    pub fn new() -> Self {
        Self { statements: vec![] }
    }
}
