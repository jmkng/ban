use crate::{
    compile::tree::{Expression, LoopVariables},
    region::Region,
};

/// Describes the state of the parser and provides temporary storage for
/// fragments of larger expressions.
pub enum State {
    /// The parser is working on an "If" expression.
    If {
        /// True if this is an "else if" expression.
        else_if: bool,
        /// Condition of the "If" tag.
        condition: Expression,
        /// Region from the containing "If" tag.
        region: Region,
        /// True if this "if" has an "else" expression.
        has_else: bool,
    },
    /// The parser is working on a "For" expression.
    For {
        /// Variables of the loop.
        variables: LoopVariables,
        /// Value being iterated on.
        iterable: Expression,
        /// Region from the containing "For" tag.
        region: Region,
    },
}

/// Describes the build state of a `Compare`.
pub enum CompareState {
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

impl Default for CompareState {
    fn default() -> Self {
        CompareState::Base(false)
    }
}
