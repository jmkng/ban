/// Describes the possible Parser states.
///
/// The Parser will store information about the structure that we
/// are building temporarily in State.
pub enum State {
    If {
        /// True if this is is an "else if".
        else_if: bool,
    },
    For {},
}
