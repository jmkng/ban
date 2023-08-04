use super::tree::{Base, Identifier, Mount};
use crate::compile::tree::{CheckTree, Set};
use std::fmt::Display;

/// Represents a fragment of a larger expression.
pub enum Fragment {
    /// The first part of an "if" block, containing a [`CheckTree`].
    If(CheckTree),
    /// An additional [`CheckTree`] provided to a parent "if" block.
    ElseIf(CheckTree),
    /// A default value for a parent "if" block.
    Else,
    /// The `(* for n in t *)` of a "for" Block.
    For(Set, Base),
    /// An assignment block - `(* let this = that *)`.
    Let(Identifier, Base),
    /// An include block - `(* include base *)`.
    Include(Base, Option<Mount>),
    /// An extends block - `(* extends base *)`.
    Extends(Base),
    /// TODO
    Block(Base),
    /// TODO
    End,
}

impl Display for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fragment::If(_) => write!(f, "if"),
            Fragment::ElseIf(_) => write!(f, "else if"),
            Fragment::Else => write!(f, "else"),
            Fragment::For(_, _) => write!(f, "for"),
            Fragment::Let(_, _) => write!(f, "let"),
            Fragment::Include(_, _) => write!(f, "include"),
            Fragment::Extends(_) => write!(f, "extends"),
            Fragment::Block(_) => write!(f, "block"),
            Fragment::End => write!(f, "end"),
        }
    }
}
