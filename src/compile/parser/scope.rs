use crate::compile::tree::Tree;

/// A distinct set of Tree instances.
#[derive(Debug, Clone)]
pub struct Scope {
    pub tokens: Vec<Tree>,
}

impl Scope {
    /// Create a new Scope.
    #[inline]
    pub fn new() -> Self {
        Self { tokens: vec![] }
    }
}
