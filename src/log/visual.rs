mod pointer;

pub use pointer::Pointer;

use std::fmt::{Debug, Formatter, Result};

const BLANK: &str = "";
const PIPE: &str = "|";
const EQUAL: &str = "=";
const HIGHLIGHT: &str = "^";

/// Describes a type that can be associated with an Error and used
/// to print a visualization.
pub trait Visual: Debug {
    /// Display the visualization by writing to the given Formatter.
    fn display(
        &self,
        formatter: &mut Formatter<'_>,
        template: Option<&str>,
        help: Option<&str>,
    ) -> Result;
}

/// Get the length and column offset for the given lines.
fn get_line_and_column(lines: &[&str], offset: usize) -> (usize, usize) {
    let mut n = 0;

    for (i, line) in lines.iter().enumerate() {
        let len = get_width(line) + 1;
        if n + len > offset {
            return (i, offset - n);
        }
        n += len;
    }

    let length = lines.len();
    let last = lines.last().map(|line| get_width(line)).unwrap_or(0);

    (length, last)
}

/// Wrapper for UnicodeWidthStr::width.
fn get_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}
