use super::tree::CheckTree;
use crate::{
    compile::tree::{Expression, Variables},
    region::Region,
};

/// Describes the internal state of a `Parser`.
pub enum BlockState {
    /// The `Parser` is working on an "if" block.
    If {
        /// True if this "if" is an "else if".
        else_if: bool,
        /// [`Compare`] derived from this "if" block.
        tree: CheckTree,
        /// Region spanning the full "if" block.
        region: Region,
        /// True if this "if" has an associated "else".
        has_else: bool,
    },
    /// The `Parser` is working on a "for" block.
    For {
        /// Variables of the loop.
        variables: Variables,
        /// Value being iterated on.
        iterable: Expression,
        /// Region spanning the full "for" tag.
        region: Region,
    },
}

/// Describes the internal state of a `CheckTree`.
pub enum CheckState {
    /// Expect a `Base`,
    ///
    /// The boolean will be true when the left (first)
    /// `Base` has already been set.
    ///
    /// When boolean is true, the "right" property will be
    /// populated with the `Base`, else the "left" property
    /// of a new `Check` will be populated.
    Base(bool),
    /// Expect an `Operator`.
    ///
    /// If a valid `Operator` is received, the "operator"
    /// property of the latest `Check` is set.
    ///
    /// If a transition such as "Operator::And", "Operator::Or"
    /// or "Token::EndBlock" is found, state will switch to
    /// `Transition` and loop will immediately reset.
    Operator,
    /// Expect `Operator::And`, `Operator::Or` or
    /// `Token::EndBlock`.
    ///
    /// Both `Operator::And` and `Operator::Or` cause a new
    /// `Check` to be started.
    ///
    /// `Token::EndBlock` will terminate the state machine.
    Transition,
}

impl Default for CheckState {
    fn default() -> Self {
        CheckState::Base(false)
    }
}
