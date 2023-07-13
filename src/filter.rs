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
//! and not a literal. Upon rendering this expression, Ash will search the
//! context for an entry with a key of "name" and use the value as the input
//! for the first filter in the chain.
//!
//! The vertical pipe "|" denotes that the following identifier is the name of
//! a filter. Ash will search for a filter with the name of "prepend" and execute
//! it with whatever "name" evaluated to as input.
//!
//! One argument for "prepend" is seen here, with a name of "1" and a value of
//! "hello, ". This is an example of a named argument, but anonymous arguments
//! are also supported.
//!
//! The next filter, "append", is using an anonymous argument.
//!
//! Anonymous arguments have no explicitly assigned name, but they do still
//! receive a name when Ash discovers them. For each anonymous argument in a
//! filter call, the name is equal to (n + 1) where n is the number of anonymous\
//! arguments which came before the argument.
//!
//! So, in the case of "append", the argument has a name of "1" and can be
//! retrieved inside of the filter like so:
//!
//! ```rs
//! args.get("1")
//! ```
use crate::error::Error;
use serde_json::Value;
use std::collections::HashMap;

/// Trait which all filter functions must implement.
///
/// The input parameter refers to the value that is being operated on,
/// args may contain any additional values passed to the filter.
///
/// ## Examples
///
/// Implementing a filter which displays a western greeting:
///
/// ```
/// use ash::{Context, Error, Filter, Value};
/// use std::collections::HashMap;
///
/// struct Cowboyify {
///     happy: bool,
/// }
///
/// impl Filter for Cowboyify {
///     fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error> {
///         let mut greeting = format!(
///             "Howdy, {}{}",
///             input.as_str().unwrap(),
///             if self.happy {
///                 "! Good to see ya! What brings you 'round these parts today?"
///             } else {
///                 ". What d'ya want?"
///             }
///         );
///
///         if !args.is_empty() {
///             greeting.push_str(" -- Well, now, ain't that a fine lookin' horse? ");
///             greeting.push_str(args.get("1").unwrap().as_str().unwrap())
///         }
///         Ok(Value::String(greeting))
///     }
/// }
///
/// // Set up the engine.
/// let mut engine = ash::new();
/// engine.add_filter_must("cowboyify", Cowboyify { happy: true });
///
/// // Build up a Context that has a "name" key.
/// let mut context = Context::new();
/// context.insert_must("name", "taylor");
///
/// // Compile the template.
/// let template = engine.compile("(( name | cowboyify \"🐴\" ))");
///
/// let expect = "Howdy, taylor! Good to see ya! What brings you 'round these parts today? \
///              -- Well, now, ain't that a fine lookin' horse? 🐴";
/// let result = engine.render(template.unwrap(), &context).unwrap();
///
/// // It worked!
/// assert_eq!(result, expect)
/// ```
pub trait Filter {
    fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error>;
}
