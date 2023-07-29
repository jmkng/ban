use super::tree::{Argument, Base, Identifier, Mount};
use crate::compile::tree::{CheckTree, Set};
use std::fmt::Display;

/// Represents a fragment of a parsed block.
pub enum Block {
    /// The `(* if x > y *)` of an "if" Block.
    If(CheckTree),
    /// The `(* else if n > m *) of an "if" Block.
    ElseIf(CheckTree),
    /// The `(* else *)` of an "if" Block.
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
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::If(_) => write!(f, "if"),
            Block::ElseIf(_) => write!(f, "else if"),
            Block::Else => write!(f, "else"),
            Block::EndIf => write!(f, "end if"),
            Block::For(_, _) => write!(f, "for"),
            Block::EndFor => write!(f, "end for"),
            Block::Let(_, _) => write!(f, "let"),
            Block::Include(_, _) => write!(f, "include"),
        }
    }
}
