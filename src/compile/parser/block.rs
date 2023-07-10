use super::tree::{Expression, LoopVariables};

/// Represents a fragment of a parsed block.
pub enum Block {
    If(Expression),
    ElseIf(Expression),
    Else,
    EndIf,
    For(LoopVariables, Expression),
    EndFor,
    EndWith,
    Include(String, Option<Expression>),
}
