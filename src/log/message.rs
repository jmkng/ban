use super::Error;
use std::fmt::Display;

pub const UNEXPECTED_TOKEN: &str = "unexpected token";
pub const UNEXPECTED_BLOCK: &str = "unexpected block";
pub const UNEXPECTED_EOF: &str = "unexpected eof";
pub const INVALID_SYNTAX: &str = "invalid syntax";
pub const INVALID_FILTER: &str = "invalid filter";
pub const INCOMPATIBLE_TYPES: &str = "incompatible types";

/// Return an [`Error`] explaining that the end of source was not expected.
pub fn error_eof(source: &str) -> Error {
    let source_len = source.len();
    Error::build(UNEXPECTED_EOF)
        .pointer(source, source_len..source_len)
        .help("expected additional tokens, did you close all blocks and expressions?")
}

/// Return an [`Error`] explaining that the write operation failed.
///
/// This is likely caused by a failure during a `write!` macro operation.
pub fn error_write() -> Error {
    Error::build("write failure").help("failed to write result of render, are you low on memory?")
}

/// Return an [`Error`] describing a missing template.
pub fn error_missing_template(name: &str) -> Error {
    Error::build("missing template").help(format!(
        "template `{}` not found in engine, add it with `.add_template`",
        name
    ))
}

/// Return a string describing an unexpected operator.
pub fn expected_operator<T>(received: T) -> String
where
    T: Display,
{
    format!(
        "expected operator like `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, found `{}`",
        received
    )
}
