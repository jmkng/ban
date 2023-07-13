use super::tree::{Expression, LoopVariables};

/// Represents a fragment of a parsed block.
pub enum Block {
    /// TODO
    If(Expression),
    /// TODO
    ElseIf(Expression),
    /// TODO
    Else,
    /// TODO
    EndIf,
    /// TODO
    For(LoopVariables, Expression),
    /// TODO
    EndFor,
    /// TODO
    EndWith,
    /// TODO
    Include(String, Option<Expression>),
}
