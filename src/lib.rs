//! A minimal and fast template engine.
//!
//! Ban is a template engine that compiles templates at runtime.
//! It supports the basic features you might expect, it's easy to use,
//! and it tries to provide good error messages.
//!
//! # Features
//! - Common logical expressions (`if`, `let`, and `for`).
//! - Output expressions with with user-defined filters.
//!     - An optional standard library providing filters for common
//!     functionality like HTML escaping.
//! - Multiple strategies for template inheritance.
//!     - Block/extends -  divide a template up into blocks that can be
//!     overridden by child templates.
//!     - Include - render another template at the specified location.
//!
//! # Usage
//!
//! Create a new instance of Engine with [`crate::new()`].
//!
//! ```rust
//! let engine = ban::new();
//! ```
//!
//! ## Compile
//!
//! Use the Engine to create a Template.
//!
//! ```rust
//! let engine = ban::new();
//! let template = engine.compile("(( name ))");
//! assert!(template.is_ok());
//! ```
//!
//! ## Create a Store
//!
//! The template has a single expression that wants to render something
//! called `name`.
//!
//! To render this, we will need to supply a Store instance that contains
//! the value of that `name` key.
//!
//! ```rust
//! use ban::Store;
//!
//! let engine = ban::new();
//! let template = engine.compile("(( name ))");
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//! ```
//!
//! ## Render
//!
//! Now that the Store instance is populated with data, the Template
//! can be rendered.
//!
//! ```rust
//! use ban::Store;
//!
//! let engine = ban::new();
//! let template = engine.compile("(( name ))");
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//!
//! let result = engine.render(&template.unwrap(), &store);
//! assert_eq!(result.unwrap(), "taylor");
//! ```
//!
//! The render method wants a reference to the compiled template,
//! because the template won't be modified or `used up` in any way.
//!
//! You may compile a single time and render it as often as necessary.
//!
//! ## Working Without an Engine
//!
//! You don't really need to create an Engine if you don't want to, Ban
//! exposes some helper methods to compile and render without one.
//!
//! You don't have to create an Engine to compile and render, Ban exposes
//! [`crate::compile()`] and [`crate::render()`] to do the same thing.
//!
//! ```rust
//! let template = ban::compile("(( name ))");
//! assert!(template.is_ok());
//!
//! let result = ban::render(&template.unwrap(), &ban::Store::new().with_must("name", "taylor"));
//! assert_eq!(result.unwrap(), "taylor");
//! ```
//!
//! The above example uses [`Store::with_must()`] to insert data instead of
//! [`Store::insert_must()`], which provides a more fluent interface.
//!
//! ## Filters
//!
//! Expressions usually render data from the Store, but you can pass in literal
//! data such as `hello` as well. This is more useful when you combine it with
//! various filters to transform the text.
//!
//! We'll create a filter that allows us to access the
//! [`to_lowercase`](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase)
//! function available in the standard library.
//!
//! All filter functions must implement this trait:
//!
//! ```rust
//! use ban::{Value, Error};
//! use std::collections::HashMap;
//!
//! pub trait Filter: Sync + Send {
//!     fn apply(&self, input: &Value, args: &HashMap<String, Value>) -> Result<Value, Error>;
//! }
//! ```
//!
//! You can either create a struct and implement the trait on that, or just create
//! a function matching the trait signature. Ban will accept both.
//!
//! Let's use a function here:
//!
//! ```rust
//! use ban::{Value, Error, Store, json};
//! use std::collections::HashMap;
//!
//! fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
//!     match value {
//!         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
//!         _ => Err(Error::build("filter `to_lowercase` requires string input")),
//!     }
//! }
//!
//! let engine = ban::new()
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
//! # Syntax
//!
//! Ban doesn't have a lot of complicated syntax to learn. It should be
//! familiar if you've used any other template engine.
//!
//! Note: This library is still in development, so this is all likely to evolve.
//!
//! ## Expressions
//!
//! Expressions let you render dynamic content from the Store.
//!
//! ```html
//! (( name ))
//! ```
//!
//! Create an use filters to transform content.
//!
//! The `source` data comes from the value on the far left, and travels
//! through each filter function from left to right. The output of the final
//! filter in the chain is what gets rendered.
//!
//! ```html
//! (( name | to_lowercase | left 3 ))
//! // "TAYLOR" - > "taylor" -> "tay"
//! ```
//!
//! Filters may accept any number of arguments, and they may be named or
//! anonymous.
//!
//!
//! Named arguments look like this:
//!
//! ```html
//! (( name | nametag name: "taylor", age: 25 ))
//! ```
//!
//! The template will attempt to find some data in the Store with
//! a key of `name`, and pass the value of that key to the filter function
//! `nametag`.
//!
//! It has two named arguments, `name` and `age`, and those arguments
//! are available in the filter function arguments hashmap. The names are
//! stored as type String, so of course they will always be treated as strings,
//! even if you pass in what looks like a number.
//!
//! Anonymous arguments work the same way, but have no explicit name:
//!
//! ```html
//! (( name | nametag "taylor", 25 ))
//! ```
//!
//! While you haven't assigned them a name, they do still receive a name,
//! which is equal to (n + 1) where `n` is the amount of anonymous arguments
//! that come before the argument.
//!
//! So, the `"taylor"` argument has a name of `"1"` and the `25` argument has a
//! name of `"2"`.
//!
//! ...
//!
//! 🚧 TODO
#![deny(unsafe_code)]
#![warn(clippy::missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod compile;
mod engine;
mod filter;
mod log;
mod pipe;
mod region;
mod render;
mod store;
mod syntax;

pub use compile::compile;
pub use filter::Filter;
pub use log::{Error, Pointer, Visual};
pub use region::Region;
pub use render::render;
pub use serde_json::{from_value, json, to_value, Value};
pub use store::Store;
pub use syntax::Builder;

pub(crate) use compile::{Parser, Scope, Template};
pub(crate) use pipe::Pipe;
pub(crate) use render::Renderer;

use engine::Engine;
use syntax::Marker;

/// Create a new instance of Ban.
pub fn new<'source>() -> Engine<'source> {
    Engine::default()
}
