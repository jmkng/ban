use super::Scope;

/// A compiled [`Template`] that can be rendered with a `Store`.
#[derive(Debug, Clone)]
pub struct Template<'source> {
    /// The name of the [`Template`].
    pub name: Option<&'source str>,
    /// The Abstract Syntax Tree generated during compilation.
    pub scope: Scope,
    /// Reference to the source data from which this [`Template`] was generated.
    pub source: &'source str,
}
