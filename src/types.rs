use std::ops::Range;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Error {
    /// Error occurred while lexing.
    Lex(String),
    /// Error occurred while parsing.
    Parse(String),
    /// Error occurred while rendering.
    Render(String),
}

#[derive(Debug, PartialEq)]
pub struct Region<T> {
    /// The data contained in the Region.
    pub data: T,
    /// The beginning and ending indices of the Region.
    pub position: Range<usize>,
}

impl<T> Region<T> {
    pub fn new(data: T, position: Range<usize>) -> Self {
        Self { data, position }
    }
}
