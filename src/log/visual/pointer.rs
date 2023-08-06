use std::{
    cmp::max,
    fmt::{Formatter, Result},
};

use super::{
    super::{RESET, YELLOW},
    {get_line_and_column, get_width, Visual, BLANK, EQUAL, HIGHLIGHT, PIPE},
};
use crate::region::Region;

/// A type of `Visual` that points to a specific location within source text.
#[derive(Debug, PartialEq)]
pub struct Pointer {
    /// The line that the Pointer is pointing to.
    ///
    /// This number should be zero indexed.
    line: usize,
    /// The column that the Pointer is pointing to.
    ///
    /// This number should be zero indexed.
    column: usize,
    /// The length of the object being highlighted.
    length: usize,
    /// The actual line of text that is being pointed to.
    text: String,
}

impl Pointer {
    /// Create a new Visual over the given source text and Region.
    pub fn new(source: &str, region: Region) -> Self {
        let lines: Vec<_> = source.split_terminator('\n').collect();
        let (line, column) = get_line_and_column(&lines, region.begin);
        let length = max(1, get_width(&source[region]));
        let text = lines
            .get(line)
            .unwrap_or_else(|| lines.last().unwrap())
            .to_string();

        Self {
            line,
            column,
            length,
            text,
        }
    }
}

impl Visual for Pointer {
    fn display(
        &self,
        formatter: &mut Formatter<'_>,
        template: Option<&str>,
        help: Option<&str>,
    ) -> Result {
        let num = (self.line + 1).to_string();
        let col = self.column + 1;
        let pad = get_width(&num);
        let align = self.column + self.length;

        let extra = "-".repeat(3_usize.saturating_sub(self.length));
        let name = template.unwrap_or("?");
        let text = &self.text;
        let underline = HIGHLIGHT.repeat(self.length);

        write!(
            formatter,
            "\n {BLANK:pad$}--> {name}:{num}:{col}\
             \n {BLANK:pad$} {PIPE}\
             \n {num:>} {PIPE} {text}\
             \n {BLANK:pad$} {PIPE} {YELLOW}{underline:>align$}{RESET}{extra}\
             \n {BLANK:pad$} {PIPE}\n",
        )?;

        if help.is_some() {
            let help = help.unwrap();
            write!(formatter, "{BLANK:pad$} {EQUAL} help: {help}\n")?;
        }

        Ok(())
    }
}
