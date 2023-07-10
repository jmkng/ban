use super::parser::scope::Scope;

/// Compiled template.
///
/// May be rendered with some context data to generate output.
#[derive(Debug)]
pub struct Template {
    pub scope: Scope,
}
