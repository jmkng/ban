use std::fmt::{Debug, Display, Formatter, Result};

use crate::{log::Visual, region::Region};

use super::{Pointer, RED, RESET};

pub const UNEXPECTED_TOKEN: &str = "unexpected token";
pub const UNEXPECTED_BLOCK: &str = "unexpected block";
pub const UNEXPECTED_EOF: &str = "unexpected eof";
pub const INCOMPATIBLE_TYPES: &str = "incompatible types";
pub const INVALID_SYNTAX: &str = "invalid syntax";
pub const INVALID_FILTER: &str = "invalid filter";

/// An error type that provides a brief description of the error,
/// and optionally supports adding more contextual "help" text and
/// a visualization to illustrate the problem.
///
/// # Examples
///
/// Creating an [`Error`] that includes a [`Visual`] of type [`Pointer`]:
///
/// ```
/// use ban::{
///     filter::{Error, Region, visual::Pointer}
/// };
///
/// let source = "(* update name *)";
/// let region = Region::new(3..9);
///
/// let error = Error::build("unexpected keyword")
///     .pointer(source, region)
///     .template("template.txt")
///     .help("expected one of \"if\", \"let\", \"for\"");
/// ```
///
/// When printed with `println!("{:#}", error)` the [`Error`] produces this output:
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
pub struct Error {
    /// Describes the cause of the [`Error`].
    reason: String,
    /// The name of the Template that the [`Error`] comes from.
    template: Option<String>,
    /// A visualization to help illustrate the [`Error`].
    visual: Option<Box<dyn Visual>>,
    /// Additional information to display with the [`Error`].
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

    /// Create a new [`Error`] with given reason text.
    ///
    /// The additional fields may be populated using the various methods
    /// defined on `Error`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::filter::Error;
    ///
    /// Error::build("unexpected keyword")
    ///     .help("expected `if`, `let` or `for`, found `...`");
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

    /// Set the reason text, which is a short summary of the [`Error`].
    pub fn reason<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.reason = text.into();
        self
    }

    /// Set the [`Template`][`crate::Template`] text, which is the name of the template
    /// that the error is related to.
    pub fn template<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.template = Some(text.into());
        self
    }

    /// Set the [`Visual`], which is a visualization that helps illustrate the
    /// cause of the error.
    pub fn visual(mut self, visual: impl Visual + 'static) -> Self {
        self.visual = Some(Box::new(visual));
        self
    }

    /// Set the visualization to a new [`Pointer`] with the given source text and
    /// [`Region`].
    ///
    /// This is a shortcut method for creating a `Pointer` yourself and then
    /// setting it to the `visual` method
    ///
    /// ```text
    /// ...
    /// error
    ///     .pointer(source, 1..2)
    /// ...
    ///
    /// // becomes:
    ///
    /// error
    ///     .pointer(source, (1..2).into())
    /// ```
    pub fn pointer<T>(mut self, source: &str, region: T) -> Self
    where
        T: Into<Region>,
    {
        self.visual = Some(Box::new(Pointer::new(source, region.into())));
        self
    }

    /// Set the help text, which is some additional contextual information
    /// to accompany the reason text which further describes the error.
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
pub fn expected_keyword(received: impl Display) -> String {
    format!(
        "expected keyword like `if`, `else`, `endif`, `let`, `for`, `in`, `endfor`, `include`, \
        `extends`, `block`, `endblock`, found `{}`",
        received
    )
}

/// Return a formatted string describing an unexpected operator.
pub fn expected_operator(received: impl Display) -> String {
    format!(
        "expected operator like `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, \
        found `{}`",
        received
    )
}

/// Return an [`Error`] explaining that the end of source was not expected
/// at this time.
pub fn error_eof(source: &str) -> Error {
    let source_len = source.len();
    Error::build(UNEXPECTED_EOF)
        .pointer(source, source_len..source_len)
        .help("expected additional tokens, did you close all blocks and expressions?")
}

pub fn error_write() -> Error {
    Error::build("write failure").help("failed to write result of render, are you low on memory?")
}
