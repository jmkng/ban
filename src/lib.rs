//! A minimal and fast template engine.
//!
//! Ban is a template engine that compiles templates at runtime.
//! It supports the basic features you might expect, it's easy to use,
//! and it tries to provide good error messages.
//!
//! # Features
//! - Common logical constructs (`if`, `let`, and `for`).
//! - User-defined filters to transform content.
//!     - An optional standard library providing filters for common
//!     functionality, like HTML escaping.
//! - Multiple strategies for template inheritance.
//!     - Block/extends - divide a template up into blocks that can be
//!     overridden by child templates.
//!     - Include - render another template at the specified location.
//! - Custom delimiters.
//!
//! ```text
//! 🦊 Note
//!
//! Ban is still in development, and is not ready to be used.
//!
//! I only document features here that are actually implemented, but even then,
//! information may be incomplete until v1.
//! ```
//!
//! ## Usage
//!
//! Create a new [`Engine`][`crate::Engine`] with [`ban::default`][`crate::default`],
//! or if you want to use custom delimiters, use the [`Builder`][`crate::Builder`]
//! type and the [`ban::new`][`crate::new`] method.
//!
//! ```
//! let engine = ban::default();
//! ```
//!
//! The [`Engine`] type provides a place for you to register filters and store other
//! templates that you can call with the `include` block.
//!
//! ## Compile
//!
//! Use the `Engine` to compile a [`Template`].
//!
//! ```
//! let engine = ban::default();
//!
//! let template = engine.compile("hello (( name ))!").is_ok();
//! ```
//!
//! ## Create a Store
//!
//! The `Template` we just compiled has a single expression that wants to render something
//! called "name".
//!
//! To render this, we will need to supply a [`Store`] instance containing a value
//! for that identifier.
//!
//! ```
//! use ban::Store;
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//! ```
//!
//! ## Render
//!
//! Now that we have a `Store` containing the data our `Template` wants to use, we can use
//! the `Engine` to render it.
//!
//! ```
//! use ban::Store;
//!
//! let engine = ban::default();
//! let template = engine.compile("hello, (( name ))!").unwrap();
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//!
//! let result = engine.render(&template, &store).unwrap();
//!
//! assert_eq!(result, "hello, taylor!");
//! ```
//!
//! # Syntax
//!
//! This section provides an overview of expressions and the different
//! blocks you can use.
//!
//! Ban should be familiar if you've used other template engines.
//!
//! ## Expressions
//!
//! Expressions let you render content from the `Store`, or literal values
//! like strings and numbers. They look like this:
//!
//! ```text
//! (( name ))
//! ```
//!
//! Or, if you want to mutate the "name" variable using filters:
//!
//! ```text
//! (( name | to_lowercase | left 3 ))
//! ```
//!
//! ## Filters
//!
//! [`Filters`][`crate::filter::Filter`] can be used in expressions to
//! transform data.
//!
//! The input for the first filter comes from the value on the far left.
//! It travels through each filter from left to right, and the output of
//! the final filter in the chain is rendered.
//!
//! Filters may accept any number of arguments, or none. They may be named
//! or anonymous.
//!
//! Named arguments require a colon between the name and value:
//!
//! ```text
//! (( name | tag name: "taylor", age: 25 ))
//! ```
//!
//! Anonymous arguments work the same way, but have no explicit name:
//!
//! ```text
//! (( name | tag "taylor", 25 ))
//! ```
//!
//! Both variants require arguments to be separated with a comma.
//!
//! See the [`filter`][`crate::filter`] module for more information.
//!
//! ## If
//!
//! If blocks allow conditional rendering based on a series of expressions.
//!
//! ```text
//! (* if true *)
//!     hello
//! (* else if false *)
//!     goodbye
//! (* end *)
//! ```
//!
//! You can compare two values, or provide just one. If the value is truthy,
//! the block will execute.
//!
//! ```text
//! (* if 100 *)
//!     hello
//! (* end *)
//! ```
//!
//! Here's a cheatsheet for truthy values:
//!
//! Type    | Truthy When
//! ------- | ----------------------------
//! String  | String is not empty.
//! Number  | Number is greater than zero.
//! Array   | Array is not empty.
//! Object  | Object is not empty.
//! Boolean | Boolean is true.
//!
//! You can use the `not` keyword to negate:
//!
//! ```text
//! (* if not false && 500 > 10 || true *)
//!     hello
//! (* end *)
//! ```
//!
//! ### Examples
//!
//! ```
//! use ban::Store;
//!
//! let mut engine = ban::default();
//! let template = engine
//!     .compile("(* if first > second *)hello(* else *)goodbye(* end *)")
//!     .unwrap();
//!
//! let store = Store::new()
//!     .with_must("first", 100)
//!     .with_must("second", 10);
//! let result = engine.render(&template, &store).unwrap();
//!
//! assert_eq!(result, "hello");
//!```
//!
//! ## For
//!
//! For blocks allow iteration over a value.
//!
//! ```text
//! (* for item in inventory *)
//!     Name: (( item.name ))
//! (* end *)
//! ```
//!
//! You can provide a single identifier as seen above, or two:
//!
//! ```text
//! (* for i, item in inventory *)
//!     Item number: (( i | add 1 )) // <-- Zero indexed, so add one!
//!     Name: (( item.name ))
//! (* end *)
//! ```
//!
//! The values held by the identifiers depends on the type you are
//! iterating on:
//!
//! Type   | Value for `i`       | Value for `item`
//! ------ | ------------------- | ----------------
//! String | Index of character. | Character of string.
//! Array  | Index of element.   | Element of array.
//! Object | Object key.         | Object value.
//!
//! ### Examples
//!
//! ```
//! use ban::{filter::serde::json, Store};
//!
//! let mut engine = ban::default();
//! let template = engine
//!     .compile("(* for item in inventory *)(( item )), (* end *)")
//!     .unwrap();
//!
//! let store = Store::new()
//!     .with_must("inventory", json!(["sword", "shield"]));
//! let result = engine.render(&template, &store).unwrap();
//!
//! assert_eq!(result, "sword, shield, ");
//!```
//!
//! ## Let
//!
//! Let expressions allow assignment of a value to an identifier.
//!
//! ```text
//! (* let name = "taylor" *)
//! ```
//!
//! The left side of the expression must be an identifier, meaning an unquoted
//! string, but the right side can be an identifier or literal value.
//!
//! ```text
//! (* if is_admin *)
//!     (* let name = "admin" *)
//! (* else *)
//!     (* let name = user.name *)
//! (* end *)
//!
//! Hello, (( name )).
//! ```
//!
//! Assignments made within a for block are scoped to the block:
//!
//! ```text
//! (* for item in inventory *)
//!     (* let name = item.name *)
//!     Name: (( name ))
//! (* end *)
//!
//! Last item name: (( name )). // <-- Error, "name" is not in scope!
//! ```
//!
//! ### Examples
//!
//! ```
//! use ban::Store;
//!
//! let mut engine = ban::default();
//!
//! let template = engine
//!     .compile("hello, (* let name = \"taylor\" -*) (( name ))!")
//!     .unwrap();
//! let store = Store::new();
//! let result = engine.render(&template, &store).unwrap();
//!
//! assert_eq!(result, "hello, taylor!");
//!```
//!
//! ## Include
//!
//! Include expressions allow other templates to be rendered.
//!
//! ```text
//! (* include header *)
//! ```
//!
//! If you call another template this way, it will have access to the same store
//! as the template that called it.
//!
//! You can pass arguments, similar to filters:
//!
//! ```text
//! (* include header name: data.name, age: data.age *)
//! ```
//!
//! When you pass arguments to an included template, it will have access to those
//! values and nothing else.
//!
//! ### Examples
//!
//! ```rust
//! use ban::{filter::serde::json, Store};
//!
//! let mut engine = ban::default();
//! engine
//!     .add_template_must("header", "hello, (( name ))! - (( age ))")
//!     .unwrap();
//!
//! let template = engine
//!     .compile(r#"(* include header name: data.name, age: data.age *)"#)
//!     .unwrap();
//!
//! let store = Store::new()
//!     .with_must("data", json!({"name": "taylor", "age": 25}));
//! let result = engine.render(&template, &store);
//!
//! assert_eq!(result.unwrap(), "hello, taylor! - 25");
//!```
//!
//! ## Extends
//!
//! Extends expressions allow templates to extend one another.
//!
//! ```text
//! (* extends parent *)
//! ```
//!
//! A template extends another template when the "extends" expression is found first
//! in the template source.
//!
//! When Ban renders an extended template, all of the blocks found in the source are
//! collected and carried to the parent. Assuming the parent template is not also
//! extended, the blocks are rendered there.
//!
//! When a block expression is found in a parent (non-extended) template, Ban will
//! render the matching block, if it has one.
//!
//! When no matching block is found, any data inside of the block is rendered instead
//! as a default value.
//!
//! ```rust
//! use ban::{Engine, Store};
//!
//! let mut engine = Engine::default();
//!
//! engine
//!     .add_template_must(
//!         "first",
//!         "hello (* block name *)(* end *), (* block greeting *)doing well?(* end *)",
//!     )
//!     .unwrap();
//! engine
//!     .add_template_must(
//!         "second",
//!         "(* extends first *)(* block name *)(( name ))(* end *)",
//!     )
//!     .unwrap();
//!
//! let store = Store::new().with_must("name", "taylor");
//! let template = engine.get_template("second").unwrap();
//!
//! assert_eq!(
//!     engine.render(&template, &store).unwrap(),
//!     "hello taylor, doing well?"
//! );
//!```
//!
//! View the [examples/inheritance](https://github.com/jmkng/ban/tree/main/examples/inheritance)
//! directory for a full illustration.
//!
//! ## Delimiters
//!
//! Use the `Builder` type to create an `Engine` that recognizes a different
//! set of delimiters.
//!
//! ```
//! use ban::{Engine, Builder, Store};
//!
//! let engine = Engine::new(
//!     Builder::new()
//!         .with_expression(">>", "<<")
//!         .with_block("{@", "@}")
//!         .with_whitespace(&'~')
//!         .to_syntax(),
//! );
//!
//! let template = engine
//!     .compile("{@ if true ~@}     Hello, >> name <<!{@ end @}")
//!     .unwrap();
//! let store = Store::new().with_must("name", "taylor");
//! let result = engine.render(&template, &store).unwrap();
//!
//! assert_eq!(result, "Hello, taylor!")
//! ```
#![doc(html_logo_url = "https://raw.githubusercontent.com/jmkng/ban/main/public/ban.svg")]
#![deny(unsafe_code)]
#![warn(clippy::missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod compile;
mod engine;
mod log;
mod region;
mod render;

pub use compile::{Builder, Template};
pub use engine::Engine;
pub use render::{filter, Store};

use morel::Syntax;

/// Create a new [`Engine`] with the given `Syntax`.
///
/// Equivalent to `Engine::new`.
///
/// # Examples
///
/// ```
/// use ban::Builder;
///
/// let syntax = Builder::new()
///     .with_expression("{{", "}}")
///     .with_block("{*", "*}")
///     .to_syntax();
///
/// let engine = ban::new(syntax);
/// ```
#[inline]
pub fn new(syntax: Syntax) -> Engine {
    Engine::new(syntax)
}

/// Create a new [`Engine`] with the default `Syntax`.
///
/// Equivalent to `Engine::default`.
///
/// # Examples
///
/// ```
/// let engine = ban::default();
/// ```
#[inline]
pub fn default() -> Engine {
    Engine::default()
}
