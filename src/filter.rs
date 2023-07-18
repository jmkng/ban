//! Contains the `Filter` trait and other types useful for creating and using filters.
//!
//! A filter is any type which implements the [`Filter`][`crate::filter::Filter`] trait.
//! You can assign a filter to an [`Engine`][`crate::Engine`] with the
//! [`add_filter`][`crate::Engine::add_filter()`] method, and it will be available in any
//! [`Template`][`crate::Template`] rendered by that engine.
//!
//! Given this expression:
//!
//! ```html
//! (( name | prepend 1: "hello, " | append "!" | upper ))
//! ```
//!
//! The "name" value is not quoted, and so it is perceived to be an identifier and not a
//! literal string. Upon rendering this expression, Ban will search the
//! [`Store`][`crate::Store`] for "name" and use that value as the input for the first
//! filter in the chain.
//!
//! The pipe "|" denotes that the following identifier is the name of a filter.
//! Ban will search for a filter with the name of "prepend" and execute it with whatever
//! "name" evaluated to.
//!
//! One argument for "prepend" is seen here with a name of "1" and a value of
//! "hello, ". This is an example of a named argument.
//!
//! The next filter, "append", is using an anonymous argument.
//!
//! Anonymous arguments have no explicitly assigned name, but they do still receive an
//! implicitly generated name. For each anonymous argument in a filter call, the name
//! is equal to (n + 1) where "n" is the number of anonymous arguments that came before the
//! argument.
//!
//! So, the "!" argument for the "append" filter will have a name of "1", because it is
//! the first anonymous argument.
//!
//! # Examples
//!
//! Expressions such as "(( name ))" usually render data from the `Store`, but you can also pass
//! in literal data like "hello". This isn't very interesting on its own, but becomes useful
//! when you start using filters.
//!
//! We'll create a filter that allows us to access the
//! [`to_lowercase`](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase)
//! function available in the standard library.
//!
//! You can either create a struct and implement the trait on that, or just create
//! a function matching the trait signature. Ban will accept both.
//!
//! Here we use a function:
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
            &engine
                .compile("(( name | to_lowercase | left 3 ))")
                .unwrap(),
            &Store::new().with_must("name", "TAYLOR"),
        );

        assert_eq!(result.unwrap(), "tay");
    }

    #[test]
    fn test_call_chain_error() {
        let engine = get_test_engine();
        let result = engine.render(
            &engine
                .compile("(( name | to_lowercase | left \"10\" ))")
                .unwrap(),
            &Store::new().with_must("name", "TAYLOR"),
        );

        // println!("{:#}", result.unwrap_err());
        assert!(result.is_err());
    }

    /// Return a new Engine equipped with test filters.
    fn get_test_engine() -> Engine<'static> {
        Engine::default()
            .with_filter_must("to_lowercase", to_lowercase)
            .with_filter_must("left", left)
    }

    /// Lowercase the given value.
    ///
    /// # Errors
    ///
    /// Returns an Error if the Value is not of type String.
    fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        match value {
            Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
            _ => Err(Error::build("filter `to_lowercase` requires string input")),
        }
    }

    /// Return the first n characters of the input Value from the left,
    /// where n is the value of the argument.
    ///
    /// Similar to TSQL `LEFT`.
    ///
    /// # Errors
    ///
    /// Returns an Error if the input is not a string, more than one
    /// argument is provided, or the argument is not a number.
    fn left(value: &Value, args: &HashMap<String, Value>) -> Result<Value, Error> {
        let arg_len = args.len();
        if arg_len != 1 {
            return Err(Error::build(format!(
                "filter `left` expects `1` argument, received `{arg_len}`"
            )));
        }

        match value {
            Value::String(string) => {
                let n = args.values().next().unwrap();

                match n {
                    Value::Number(number) => match number.as_u64() {
                        Some(u64) => {
                            let n_left = string.chars().take(u64 as usize).collect::<String>();
                            Ok(json!(n_left))
                        }
                        None => Err(Error::build(format!(
                            "filter `left` expects an integer (not a float) that fits in u64, \
                            `{}` is invalid",
                            number
                        ))),
                    },
                    _ => Err(Error::build(format!(
                        "filter `left` expects a number argument, received `{}`",
                        n,
                    ))),
                }
            }
            _ => Err(Error::build("filter `left` expects string input")),
        }
    }
}
