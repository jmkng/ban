mod error;
mod visual;

pub use error::{
    expected_keyword, expected_operator, Error, INVALID_FILTER, INVALID_SYNTAX, UNDELIMITED_STRING,
    UNEXPECTED_CHAR, UNEXPECTED_EOF, UNEXPECTED_TOKEN,
};
pub use visual::{Pointer, Visual};

const RED: &str = "\x1B[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1B[0m";
