use std::ops::Range;

#[derive(Debug, PartialEq)]
pub enum Error {
    /// Error occurred while lexing.
    Lex(String),
    /// Error occurred while parsing.
    Parse(String),
    /// Error occurred while rendering.
    Render(String),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Region<T: Copy + Clone> {
    /// The data contained in the Region.
    pub data: T,
    /// The beginning and ending indices of the Region.
    pub begin: usize,
    pub end: usize,
}

impl<T: Copy + Clone> Into<Range<usize>> for Region<T> {
    fn into(self) -> Range<usize> {
        self.begin..self.end
    }
}

impl<T: Copy + Clone> Region<T> {
    pub fn new(data: T, position: Range<usize>) -> Self {
        Self {
            data,
            begin: position.start,
            end: position.end,
        }
    }
}
