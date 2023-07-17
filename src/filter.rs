//! Declares the Filter trait and documents the creation and usage of filters.
//!
//! A filter is any type which implements the Filter trait seen below.
//! Filter instances may be assigned to an Engine with the [.add_filter()] method,
//! and will be available in any template which is rendered by that Engine instance.
//!
//! Given this expression:
//!
//! ```html
//! (( name | prepend 1: "hello, " | append "!" | upper ))
//! ```
//!
//! The "name" value is not quoted, and so it is perceived to be an identifier
//! and not a literal. Upon rendering this expression, Ban will search the
//! Store for an entry with a key of "name" and use the value as the input
//! for the first filter in the chain.
//!
//! The vertical pipe "|" denotes that the following identifier is the name of
//! a filter. Ban will search for a filter with the name of "prepend" and execute
//! it with whatever "name" evaluated to as input.
//!
//! One argument for "prepend" is seen here, with a name of "1" and a value of
//! "hello, ". This is an example of a named argument, but anonymous arguments
//! are also supported.
//!
//! The next filter, "append", is using an anonymous argument.
//!
//! Anonymous arguments have no explicitly assigned name, but they do still
//! receive a name when Ban discovers them. For each anonymous argument in a
//! filter call, the name is equal to (n + 1) where n is the number of anonymous\
//! arguments which came before the argument.
//!
//! So, in the case of "append", the argument has a name of "1" and can be
//! retrieved inside of the filter like so:
//!
//! ```rs
//! args.get("1")
//! ```
pub use crate::store::Store;

use crate::{Error, Value};
use std::collections::HashMap;

/// Describes a type which can be created and stored within an Engine,
/// and used to transform input in an Expression.
///
/// The input parameter refers to the value that is being operated on,
/// args may contain any additional values passed to the filter.
///
/// ## Examples
///
/// Implementing a filter which returns the lowercase equivalent of a string:
///
/// ```
/// use ban::{json, Error, Filter, Store, Value};
/// use std::collections::HashMap;
///
/// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
///     match value {
///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
///         _ => Err(Error::build("filter `to_lowercase` requires string input")),
///     }
/// }
///
/// let mut engine = ban::new()
///     .with_filter_must("to_lowercase", to_lowercase);
///
/// let result = engine.render(
///     &engine.compile_must("(( name | to_lowercase ))"),
///     &Store::new().with_must("name", "TAYLOR"),
/// );
///
/// assert_eq!(result.unwrap(), "taylor")
/// ```
pub trait Filter: Sync + Send {
    /// Execute the filter with the given input, and return a new Value as output.
    fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error>;
}

// Allows assignment of any function matching the signature of `apply` as a Filter
// to Engine, instead of requiring a struct be created.
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
    use crate::{engine::Engine, json, store::Store, Error, Value};
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
