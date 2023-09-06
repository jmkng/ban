use super::{tree::Extends, Scope};

/// A compiled [`Template`] that can be rendered against a [`Store`][`crate::Store`].
#[derive(Debug, Clone)]
pub struct Template {
    /// The name of the [`Template`].
    name: Option<String>,
    /// The Abstract Syntax Tree generated during compilation.
    scope: Scope,
    /// The source data from which this [`Template`] was generated.
    source: String,
    /// If the [`Template`] is extended, contains information about the other
    /// `Template`.
    extends: Option<Extends>,
}

impl Template {
    /// Create a new [`Template`].
    pub(crate) fn new(
        name: Option<String>,
        scope: Scope,
        source: String,
        extends: Option<Extends>,
    ) -> Self {
        Self {
            name,
            scope,
            source,
            extends,
        }
    }

    /// Return a reference to the name of the [`Template`].
    #[inline]
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_ref().map(|x| x.as_str())
    }

    /// Return a reference to the source text of the [`Template`].
    #[inline]
    pub fn get_source(&self) -> &str {
        &self.source
    }

    /// Return a reference to the [`Extends`] of the [`Template`].
    #[inline]
    pub(crate) fn get_extends(&self) -> Option<&Extends> {
        self.extends.as_ref()
    }

    /// Return a reference to the [`Scope`] of the [`Template`].
    #[inline]
    pub(crate) fn get_scope(&self) -> &Scope {
        &self.scope
    }
}
