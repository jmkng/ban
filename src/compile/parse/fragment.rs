use std::fmt::Display;

use crate::compile::tree::{IfTree, Set};

use super::tree::{Base, Expression, Identifier, Mount};

/// Represents a fragment of a larger expression.
pub enum Fragment {
    /// The first part of an "if" block, containing a [`IfTree`].
    If(IfTree),
    /// An additional [`IfTree`] provided to a parent "if" block.
    ElseIf(IfTree),
    /// A default value for a parent "if" block.
    Else,
    /// The first part of an "if" block, containing a set of faux
    /// scope variables and something to iterate over.
    For(Set, Base),
    /// A "let" expression, used in assignment operations.
    Let(Identifier, Expression),
    /// An "include" expression, used to render other templates in place.
    Include(Base, Option<Mount>),
    /// An "extends" expression, tells the `Renderer` handling the
    /// `Template` to carry blocks up to a parent.
    ///
    /// Must be found at the top of a `Template`.
    Extends(Base),
    /// A "block" expression, defines an area that can be overridden by
    /// another extending template.
    Block(Base),
    /// Closes a block.
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
