use std::fmt::Display;

#[macro_export]
macro_rules! general_error {
    ($fmt:expr $(, $($args:expr),*)?) => {
        Err(Error::General(format!($fmt $(, $($args),*)?)))
    };
}

/// Represents a custom error type.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// A general purpose, non-specific error.
    General(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::General(s) => write!(f, "{s}"),
        }
    }
}
