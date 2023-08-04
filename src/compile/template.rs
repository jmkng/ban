use super::{tree::Extends, Scope};

/// A compiled [`Template`] that can be rendered with a [`Store`][`crate::Store`].
#[derive(Debug, Clone)]
pub struct Template {
    /// The name of the [`Template`].
    pub(crate) name: Option<String>,
    /// The Abstract Syntax Tree generated during compilation.
    pub(crate) scope: Scope,
    /// Reference to the source data from which this [`Template`] was generated.
    pub(crate) source: String,
    /// If the [`Template`] is extended, contains information about the other
    /// `Template`.
    pub(crate) extended: Option<Extends>,
}
