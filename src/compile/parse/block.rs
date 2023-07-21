use crate::compile::tree::{Compare, Expression, LoopVariables};

/// Represents a fragment of a parsed block.
pub enum Block {
    /// The `(* if x > y *)` part of an "if" Block.
    If(Compare),
    /// The `(* elseif n > m *) part of an "if" Block.
    ElseIf(Compare),
    /// The (* else *) part of an "if" Block.
    Else,
    /// The (* endif *) part of an "if" Block.
    EndIf,
    /// The (* for n in t *) part of a "for" Block.
    For(LoopVariables, Expression),
    /// The "(* endfor *)" part of a "for" Block.
    EndFor,
    /// TODO
    Include(String, Option<Expression>),
}
