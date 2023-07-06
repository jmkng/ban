use std::fmt::Display;

/// Ash error type.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// General, non-specific error.
    General(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::General(s) => write!(f, "{s}"),
        }
    }
}
