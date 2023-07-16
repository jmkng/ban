use crate::Scope;

/// Compiled template.
///
/// May be rendered with some Store data to generate output.
#[derive(Debug, Clone)]
pub struct Template<'source> {
    /// The Abstract Syntax Tree generated during compilation.
    pub scope: Scope,
    /// Reference to the source data from which this template was generated.
    pub source: &'source str,
}
