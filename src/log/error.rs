use super::{Pointer, RED, RESET};
use crate::{log::Visual, region::Region};
use std::fmt::{Debug, Display, Formatter, Result};

/// Describes an error, and allows adding a contextual help text and visualization.
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
/// Error::build("unexpected keyword")
///     .with_pointer("(* update name *)", Region::new(3..9))
///     .with_name("template.txt")
///     .with_help(r#"expected one of "if", "let", "for""#);
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
    /// A visualization to help illustrate the [`Error`].
    visual: Option<Box<dyn Visual>>,
    /// Additional information to display with the [`Error`].
    help: Option<String>,
    /// The name of the Template that the [`Error`] comes from.
    name: Option<String>,
}

impl Error {
    /// Create a new [`Error`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{filter::{visual::Pointer, Error}};
    ///
    /// let pointer = Pointer::new("source", (0..4).into());
    /// Error::new("unexpected keyword", "name", "help", pointer);
    /// ```
    pub fn new<T, Y>(reason: T, name: T, help: T, visual: Y) -> Self
    where
        T: Into<String>,
        Y: Visual + 'static,
    {
        Error {
            reason: reason.into(),
            name: Some(name.into()),
            visual: Some(Box::new(visual)),
            help: Some(help.into()),
        }
    }

    /// Create a new [`Error`] with the given reason text.
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
    ///     .with_help("expected `if`, `let` or `for`, found `...`");
    /// ```
    pub fn build<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        Error {
            reason: reason.into(),
            name: None,
            visual: None,
            help: None,
        }
    }

    /// Set the reason text, which is a short summary of the [`Error`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::filter::Error;
    ///
    /// // Reason text begins as an empty string, but is immediately
    /// // updated to "something else":
    /// let mut error = Error::build("")
    ///     .with_reason("something else");
    /// ```
    pub fn with_reason<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.reason = text.into();

        self
    }

    /// Set the name text, which is the name of the [`Template`][`crate::Template`]
    /// that the [`Error`] is related to.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::filter::Error;
    ///
    /// let mut error = Error::build("unexpected keyword")
    ///     .with_reason("something else");
    /// ```
    pub fn with_name<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.name = Some(text.into());

        self
    }

    /// Set the [`Visual`], which is a visualization that helps illustrate the
    /// cause of the error.
    pub fn with_visual(mut self, visual: impl Visual + 'static) -> Self {
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
    ///     .visual(Pointer::new(source, (1..2).into()))
    /// ...
    ///
    /// // becomes:
    ///
    /// error
    ///     .pointer(source, (1..2).into())
    /// ```
    pub fn with_pointer<T>(mut self, source: &str, region: T) -> Self
    where
        T: Into<Region>,
    {
        self.visual = Some(Box::new(Pointer::new(source, region.into())));

        self
    }

    /// Set the help text, which is contextual information to accompany the
    /// reason text.
    pub fn with_help<T>(mut self, text: T) -> Self
    where
        T: Into<String>,
    {
        self.help = Some(text.into());

        self
    }

    /// Return the name of the `Template` that the error is related to.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_ref().map(|x| x.as_str())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !f.alternate() {
            writeln!(f, "{self:#}")?;
        }
        f.debug_struct("Error")
            .field("reason", &self.reason)
            .field("name", &self.name)
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
                self.name.as_deref(),
                self.help.as_deref(),
            );
        }

        Ok(())
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason && self.help == other.help && self.name == other.name
    }
}
