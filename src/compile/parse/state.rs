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
