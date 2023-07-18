use super::Scope;

/// A compiled template that can be rendered with a `Store`.
#[derive(Debug, Clone)]
pub struct Template<'source> {
    /// The Abstract Syntax Tree generated during compilation.
    pub scope: Scope,
    /// Reference to the source data from which this template was generated.
    pub source: &'source str,
}
