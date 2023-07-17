use std::fmt::{Debug, Display, Formatter, Result};

use crate::{compile::token::Token, Visual};

use super::{RED, RESET};

pub const UNEXPECTED_TOKEN: &str = "unexpected token";
pub const UNEXPECTED_EOF: &str = "unexpected eof";
pub const INVALID_SYNTAX: &str = "invalid syntax";
pub const INVALID_FILTER: &str = "invalid filter";

/// An error type that optionally supports printing a visualization.
pub struct Error {
    /// Describes the cause of the Error.
    reason: String,
    /// The name of the Template that the Error comes from.
    template: Option<String>,
    /// A visualization to help illustrate the Error.
    visual: Option<Box<dyn Visual>>,
    /// Additional information to display with the Error.
    help: Option<String>,
}

impl Error {
    /// Create a new Error.
    pub fn new<T>(reason: T, template: T, help: T, visual: impl Visual + 'static) -> Self
    where
        T: Into<String>,
    {
        Error {
            reason: reason.into(),
            template: Some(template.into()),
            visual: Some(Box::new(visual)),
            help: Some(help.into()),
        }
    }

    /// Create a new Error with given reason text.
    ///
    /// The remaining values may be updated using the builder API.
    ///
    /// # Examples
    ///
    /// Creating an Error with the builder API that includes a Visual
    /// of type Pointer:
    ///
    /// ```rs
    /// use ban::{Region, Pointer, Error};
    ///
    /// let source = "(* update name *)";
    /// let region = Region::new(3..9);
    ///
    /// let error = Error::build("unexpected keyword")
    ///     .visual(Pointer::new(source, region))
    ///     .template("template.txt")
    ///     .help("expected one of \"if\", \"let\", \"for\"");
    /// ```
    ///
    /// When printed with `println!("{:#}", error)` the error produces this output:
    ///
    /// ```text
    /// error: unexpected keyword
    ///   --> template.txt:1:4
    ///    |
    ///  1 | (* update name *)
    ///    |    ^^^^^^
    ///    |
    ///   = help: expected one of "if", "let", "for"
    /// ```
    pub fn build<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        Error {
            reason: reason.into(),
            template: None,
            visual: None,
            help: None,
        }
    }

    /// Set the reason text, which is a short summary of the Error.
    pub fn reason<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.reason = text.into();
        self
    }

    /// Set the template text, which is the name of the template that the
    /// Error is related to.
    pub fn template<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.template = Some(text.into());
        self
    }

    /// Set the visualization, which is a Visual that helps illustrate the
    /// cause of the Error.
    pub fn visual(mut self, visual: impl Visual + 'static) -> Self {
        self.visual = Some(Box::new(visual));
        self
    }

    /// Set the help text, which is some additional contextual information
    /// to accompany the reason text which describes the Error in more detail.
    pub fn help<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.help = Some(text.into());
        self
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !f.alternate() {
            writeln!(f, "{self:#}")?;
        }

        f.debug_struct("Error")
            .field("reason", &self.reason)
            .field("template", &self.template)
            .field("visual", &self.visual)
            .field("help", &self.help)
            .finish()?;
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let header = format!("{RED}error{RESET}");
        write!(f, "{header}: {}", self.reason)?;

        if self.visual.is_some() && f.alternate() {
            return self.visual.as_ref().unwrap().display(
                f,
                self.template.as_deref(),
                self.help.as_deref(),
            );
        }

        Ok(())
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason && self.help == other.help && self.template == other.template
    }
}

/// Return a formatted string describing an unexpected keyword.
pub fn expected_keyword(received: Token) -> String {
    format!(
        "expected keyword like \"if\", \"let\", or \"for\", found {}",
        received
    )
}

/// Return a formatted string describing an unexpected operator.
pub fn expected_operator(received: char) -> String {
    format!(
        "expected operator like `==`, `!=, `>=`, `<=`, `||`, `&&`, `=`, `|`, `!`, \
        found {}",
        received
    )
}

#[cfg(test)]
mod tests {
    use crate::{log::visual::Pointer, Region};

    use super::*;

    #[test]
    fn test_syntax_error() {
        let source = "(* update name *)";
        let region = Region::new(3..9);

        let error = Error::build("unexpected keyword")
            .visual(Pointer::new(source, region))
            .template("template.txt")
            .help("expected one of \"if\", \"let\", \"for\"");

        // println!("{:#}", error)
    }
}
