use super::tree::{Base, Identifier, Mount};
use crate::compile::tree::{CheckTree, Set};
use std::fmt::Display;

/// Represents a fragment of a larger expression.
pub enum Block {
    /// The first part of an "if" block, containing a [`CheckTree`].
    If(CheckTree),
    /// An additional [`CheckTree`] provided to a parent "if" block.
    ElseIf(CheckTree),
    /// A default value for a parent "if" block.
    Else,
    /// The `(* endif *)` of an "if" Block.
    EndIf,
    /// The `(* for n in t *)` of a "for" Block.
    For(Set, Base),
    /// The `(* endfor *)` of a "for" Block.
    EndFor,
    /// An assignment block - `(* let this = that *)`.
    Let(Identifier, Base),
    /// An include block - `(* include base *)`.
    Include(Base, Option<Mount>),
    /// An extends block - `(* extends base *)`.
    Extends(Base),
    /// TODO
    Block(Base),
    /// TODO
    EndBlock,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::If(_) => write!(f, "if"),
            Block::ElseIf(_) => write!(f, "else if"),
            Block::Else => write!(f, "else"),
            Block::EndIf => write!(f, "endif"),
            Block::For(_, _) => write!(f, "for"),
            Block::EndFor => write!(f, "endor"),
            Block::Let(_, _) => write!(f, "let"),
            Block::Include(_, _) => write!(f, "include"),
            Block::Extends(_) => write!(f, "extends"),
            Block::Block(_) => write!(f, "block"),
            Block::EndBlock => write!(f, "endblock"),
        }
    }
}
