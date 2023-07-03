use std::ops::Range;

/// Ash error type.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Error occurred while lexing.
    Lex(String),
    /// Error occurred while parsing.
    Parse(String),
    /// Error occurred while rendering.
    Render(String),
}

/// Represents a region (beginning and ending indices) within some source.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Region {
    /// The beginning of the range, inclusive.
    pub begin: usize,
    /// The ending of the range, exclusive.
    pub end: usize,
}

impl Region {
    pub fn new(position: Range<usize>) -> Self {
        Self {
            begin: position.start,
            end: position.end,
        }
    }
}

impl From<Range<usize>> for Region {
    fn from(value: Range<usize>) -> Self {
        Self {
            begin: value.start,
            end: value.end,
        }
    }
}

impl From<Region> for Range<usize> {
    fn from(value: Region) -> Self {
        Self {
            start: value.begin,
            end: value.end,
        }
    }
}
