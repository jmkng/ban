//! Contains the [`Filter`] trait and other types useful for creating and using
//! filters.
//!
//! A filter is any type that implements the [`Filter`] trait. You can add a
//! filter to an [`Engine`][`crate::Engine`] with the
//! [`.add_filter`][`crate::Engine::add_filter`] method, and it will be available
//! in any [`Template`][`crate::Template`] rendered by that engine.
//!
//! ```html
//! (( name | prepend text: "hello, " | append "!" | upper ))
//! ```
//!
//! Upon rendering this expression, Ban will search the [`Store`][`crate::Store`]
//! for "name" and use that value as the input for the first filter in the chain.
//!
//! The `prepend` filter receives one argument with a name of "text", and the value
//! for that argument is "hello, ".
//!
//! It might be important to note, argument names may be quoted or unquoted, they
//! are treated the same either way.
//!
//! The next filter, `append`, receives an anonymous argument with a value of "!".
//!
//! Anonymous arguments have no explicitly assigned name, but they do still receive
//! an implicitly generated name. For each anonymous argument to a filter, the name
//! is equal to `n + 1` where "n" is the number of anonymous arguments that came
//! before the argument.
//!
//! So, the "!" argument receives a name of "1" because it is the first anonymous
//! argument.
//!
//! # Examples
//!
//! Expressions such as `(( name ))` usually render data from the `Store`, but you
//! can also pass in literal data like "hello". This isn't very interesting on its own,
//! but becomes useful when you start using filters.
//!
//! We'll create a filter that allows us to access the
//! [`to_lowercase`](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase)
//! function available in the standard library.
//!
//! You can either create a struct and implement the trait on it, or just create
//! a function matching the trait signature:
//!
//! ```rust
//! use ban::{
//!     filter::{
//!         serde::{json, Value},
//!         Error,
//!     },
//!     Store,
//! };
//! use std::collections::HashMap;
//!
//! fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
//!     match value {
//!         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
//!         _ => Err(Error::build("filter `to_lowercase` requires string input")
//!                 .help("use quotes to coerce data to string")
//!              ),
//!     }
//! }
//!
//! let engine = ban::default()
//!     .with_filter_must("to_lowercase", to_lowercase);
//!
//! let template = engine.compile("(( name | to_lowercase ))");
//!
//! let result = engine.render(
//!     &template.unwrap(),
//!     &Store::new()
//!         .with_must("name", "TAYLOR")
//! );
//!
//! assert_eq!(result.unwrap(), "taylor");  
//! ```
//!
//! If you return an [`Error`][`crate::filter::Error`] in your filter without using the
//! [`visual`][`crate::filter::Error::visual`] method to set your own visualization,
//! Ban will automatically generate one that points to the filter.
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
//! If you don't want the visulization to be shown, simply print the error with `{}`
//! instead:
//!
//! ```text
//! error: filter `to_lowercase` requires string input
//! ```

pub mod serde {
    //! Contains types from `serde_json`.
    pub use serde_json::*;
}
pub mod visual {
    //! Contains the `Visual` trait and different types which implement `Visual`.
    pub use crate::log::{Pointer, Visual};
}

pub use crate::{log::Error, region::Region};

use serde_json::Value;
use std::collections::HashMap;

/// Describes a type which can be used to transform input in an expression.
pub trait Filter: Sync + Send {
    /// Execute the filter with the given input and return a new Value as output.
    fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error>;
}

/// Allows assignment of any function matching the signature of `apply` as a `Filter`
/// to `Engine`, instead of requiring a struct be created.
impl<F> Filter for F
where
    F: Fn(&Value, &HashMap<String, Value>) -> Result<Value, Error> + Sync + Send,
{
    fn apply(&self, value: &Value, args: &HashMap<String, Value>) -> Result<Value, Error> {
        self(value, args)
    }
}

#[cfg(test)]
mod tests {
    use crate::{engine::Engine, log::Error, store::Store};
    use serde_json::{json, Value};
    use std::collections::HashMap;

    #[test]
    fn test_call_chain() {
        let engine = get_test_engine();
        let result = engine.render(
            &engine.compile("(( name | to_lowercase ))").unwrap(),
            &Store::new().with_must("name", "TAYLOR"),
        );

        assert_eq!(result.unwrap(), "taylor");
    }

    /// Return a new Engine equipped with test filters.
    fn get_test_engine() -> Engine {
        Engine::default().with_filter_must("to_lowercase", to_lowercase)
    }

    /// Lowercase the given value.
    ///
    /// # Errors
    ///
    /// Returns an error if the Value is not of type String.
    fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        match value {
            Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
            _ => Err(Error::build("filter `to_lowercase` requires string input")),
        }
    }
}
