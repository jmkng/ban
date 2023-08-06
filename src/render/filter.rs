//! Contains the [`Filter`] trait, and types useful for creating and using
//! filters.
//!
//! A `Filter` is a user-defined function that can be used to modify a [`Value`]
//! before it is rendered. Any struct that implements the [`Filter`] trait,
//! or function matching the [`apply`][`Filter::apply`] method, can be registered as a
//! `Filter` on an [`Engine`][`crate::Engine`].
//!
//! A `Filter` registered with an `Engine` is available for use in any
//! [`Template`][`crate::Template`] rendered by that `Engine`.
//!
//! ## Examples
//!
//! This expression attempts to render a "name" variable from the [`Store`][`crate::Store`],
//! but you can also pass in literal values like strings and numbers.
//!
//! ```html
//! (( name | prepend text: "hello, " | append "!", "?" | upper ))
//! ```
//!
//! Upon rendering this expression, Ban will search the `Store` for "name" and use that
//! value as the input for `prepend`, the first filter in the call chain.
//!
//! The `prepend` filter receives one named argument with a name of "text" and value
//! of "hello, ". Argument names may be quoted or unquoted.
//!
//! The next filter, `append`, receives two anonymous arguments; "!" and "?".
//!
//! Anonymous arguments have no explicitly assigned name, but they do still receive
//! names. For each anonymous argument to a filter, the name is equal to `n + 1`
//! where "n" is the number of anonymous arguments that came before the argument.
//!
//! So, the "!" argument receives a name of "1" because it is the first anonymous
//! argument.
//!
//! We'll create a filter that allows us to access the
//! [`to_lowercase`](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase)
//! function available in the standard library.
//!
//! You can either create a struct and implement the trait on it, or just create
//! a function matching the trait signature:
//!
//! ```
//! use std::collections::HashMap;
//!
//! use ban::{
//!     filter::{
//!         serde::{json, Value},
//!         Error,
//!     },
//!     Store,
//! };
//!
//! fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
//!     match value {
//!         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
//!         _ => Err(Error::build("filter `to_lowercase` requires string input")
//!                 .with_help("use quotes to coerce data to string")
//!              ),
//!     }
//! }
//!
//! let engine = ban::default()
//!     .with_filter_must("to_lowercase", to_lowercase);
//! let template = engine.compile("(( name | to_lowercase ))").unwrap();
//! let result = engine.render(
//!     &template,
//!     &Store::new()
//!         .with_must("name", "TAYLOR")
//! ).unwrap();
//!
//! assert_eq!(result, "taylor");  
//! ```
//!
//! The [`with_visual`][`crate::filter::Error::visual`] method can be used to create a
//! visualization when your `Filter` needs to return an [`Error`]. If you don't set one
//! yourself, Ban will automatcially assign a [`Pointer`][`crate::filter::visual::Pointer`]
//! that points to the `Filter` name.
//!
//! If you were to pass a number to the filter and print the error with `{:#}`,
//! you would see:
//!
//! ```text
//!  error: filter `to_lowercase` requires string input
//!  --> ?:1:11
//!   |
//! 1 | (( name | to_lowercase ))
//!   |           ^^^^^^^^^^^^
//!   |
//!  = help: use quotes to coerce data to string
//! ```
//!
//! If you don't want the visulization to be shown, print the error with `{}` instead:
//!
//! ```text
//! error: filter `to_lowercase` requires string input
//! ```

pub mod serde {
    //! Contains types from `serde_json`.
    pub use serde_json::*;
}
pub mod visual {
    //! Contains the `Visual` trait and types that implement `Visual`.
    pub use crate::log::{Pointer, Visual};
}

use std::collections::HashMap;

pub use crate::{log::Error, region::Region};

use serde_json::Value;

pub const INVALID_FILTER: &str = "invalid filter";

/// Describes a type that can be used to mutate a [`Value`].
pub trait Filter: Sync + Send {
    /// Apply the [`Filter`] with the given input [`Value`] and arguments,
    /// and return a new `Value`.
    ///
    /// # Errors
    ///
    /// May return an [`Error`] to abort template rendering.
    fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error>;
}

/// Allows any function with a matching signature to be registered as a [`Filter`].
impl<F> Filter for F
where
    F: Fn(&Value, &HashMap<String, Value>) -> Result<Value, Error> + Sync + Send,
{
    fn apply(&self, value: &Value, args: &HashMap<String, Value>) -> Result<Value, Error> {
        self(value, args)
    }
}
