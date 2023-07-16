use crate::compile::tree::Tree;

/// A distinct set of Tree instances.
#[derive(Debug, Clone)]
pub struct Scope {
    pub data: Vec<Tree>,
}

impl Scope {
    /// Create a new Scope.
    #[inline]
    pub fn new() -> Self {
        Self { data: vec![] }
    }
}
