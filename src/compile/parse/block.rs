use super::tree::{Base, Identifier};
use crate::compile::tree::{CheckTree, Set};
use std::fmt::Display;

/// Represents a fragment of a parsed block.
pub enum Block {
    /// The `(* if x > y *)` part of an "if" Block.
    If(CheckTree),
    /// The `(* else if n > m *) part of an "if" Block.
    ElseIf(CheckTree),
    /// The (* else *) part of an "if" Block.
    Else,
    /// The (* endif *) part of an "if" Block.
    EndIf,
    /// The (* for n in t *) part of a "for" Block.
    For(Set, Base),
    /// The "(* endfor *)" part of a "for" Block.
    EndFor,
    /// An assignment block such as "(* let this = that *)".
    Let(Identifier, Base),
    /// TODO
    Include(String, Option<Base>),
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
