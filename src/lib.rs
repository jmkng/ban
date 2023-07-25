//! A minimal and fast template engine.
//!
//! Ban is a template engine that compiles templates at runtime.
//! It supports the basic features you might expect, it's easy to use,
//! and it tries to provide good error messages.
//!
//! # Features
//! - Common logical expressions (`if`, `let`, and `for`).
//! - Output expressions with user-defined filters.
//!     - An optional standard library providing filters for common
//!     functionality like HTML escaping.
//! - Multiple strategies for template inheritance.
//!     - Block/extends - divide a template up into blocks that can be
//!     overridden by child templates.
//!     - Include - render another template at the specified location.
//!
//! ```text
//! 🦊 Note
//!
//! Ban is still in development and is not ready to be used.
//!
//! I only document features here that are actually implemented, but even then,
//! information may be incomplete until v1.
//! ```
//!
//! ## Usage
//!
//! Create a new [`Engine`][`crate::Engine`] with [`default`][`crate::default`],
//! or if you want to use custom delimiters, create a `Syntax` with
//! [`Builder`][`crate::Builder`] first and use the [`new`][`crate::Engine::new`]
//! method instead.
//!
//! ```rust
//! let engine = ban::default();
//! ```
//!
//! The `Engine` type provides a place for you to register filters and store
//! other templates that you can use with the `include` expression.
//!
//! ## Compile
//!
//! Use the `Engine` to compile a [`Template`][`crate::Template`].
//!
//! ```rust
//! let engine = ban::default();
//! let template = engine.compile("hello (( name ))!");
//! ```
//!
//! ## Create a Store
//!
//! The template we just compiled has a single expression that wants to render something
//! called "name".
//!
//! To render this, we will need to supply a [`Store`][`crate::Store`] instance that contains
//! a value for that "name" key.
//!
//! ```rust
//! use ban::Store;
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//! ```
//!
//! ## Render
//!
//! Now that we have a `Store` containing the data our `Template` wants
//! to use, we can tell the `Engine` to render it for us.
//!
//! ```rust
//! use ban::Store;
//!
//! let engine = ban::default();
//! let template = engine.compile("hello, (( name ))!");
//!
//! let mut store = Store::new();
//! store.insert_must("name", "taylor");
//!
//! let result = engine.render(&template.unwrap(), &store);
//! assert_eq!(result.unwrap(), "hello, taylor!");
//! ```
//!
//! ## Working Without an Engine
//!
//! You don't have to create an `Engine` to compile and render, Ban exposes
//! [`compile`][`crate::compile()`] and [`render`][`crate::render()`] as a shortcut.
//!
//! ```rust
//! use ban::{compile, render, Store};
//!
//! let template = compile("hello, (( name ))!");
//! let result = render(
//!     &template.unwrap(),
//!     &Store::new().with_must("name", "taylor")
//! );
//!
//! assert_eq!(result.unwrap(), "hello, taylor!");
//! ```
//!
//! However, because the `Engine` contains all of the `Filter` instances,
//! working this way means that you will not have the ability to use
//! custom filters.
//!
//! Another thing you might have noticed, the above example uses
//! [`with_must`][`Store::with_must`] to insert data instead of
//! [`insert_must`][`Store::insert_must`]. This method does the same thing,
//! but provides a more fluent interface.
//!
//! # Syntax
//!
//! Ban doesn't have a lot of complicated syntax to learn. It should be
//! familiar if you've used any other template engine.
//!
//! This section provides an overview of expressions, and the different
//! blocks you can use.
//!
//! ## Expressions
//!
//! Expressions let you render content from the `Store`, or literal values
//! such as strings and numbers. They look like this:
//!
//! ```html
//! (( name | to_lowercase | left 3 ))
//! ```
//!
//! You can use filters to transform content in expressions.
//!
//! The input for the first filter comes from the value on the far left.
//! It travels through each filter from left to right, and the output of
//! the final filter in the chain is what gets rendered.
//!
//! Filters may accept any number of arguments, or none. They may be named or anonymous.
//!
//! Named arguments look like this:
//!
//! ```html
//! (( name | tag name: "taylor", age: 25 ))
//! ```
//!
//! Anonymous arguments work the same way, but have no explicit name:
//!
//! ```html
//! (( name | tag "taylor", 25 ))
//! ```
//!
//! See the [`filter`][`crate::filter`] module for more information.
//!
//! ## If
//!
//! If blocks allow conditional rendering based on a series of expressions.
//!
//! In this example, none of the "branches" are found to be "truthy", so
//! nothing gets rendered. The last branch *would* be considered "truthy",
//! but it is negated with the `not` keyword.
//!
//! ```html
//! (* if false || 10 > 500 || not true *)
//!     hello
//! (* endif *)
//! ```
//!
//! You can also provide a single identifier or literal, and it will pass if it
//! is truthy.
//!
//! ```html
//! (* if 100 *)
//!     hello
//! (* endif *)
//! ```
//!
//! An if block is represented by a "tree", which may have one or more "branches",
//! and each branch may have one or more "checks".
//!
//! The "||" and "&&" operators are used to divide branches and checks respectively,
//! similar to how they are used in many programming languages.
//!
//! By using "&&", you are declaring that you intend to write another check and that
//! it should be associated with the same branch, while "||" begins a new branch.
//!
//! ```html
//!                                        |---| negated
//! (* if this >= that && these == those || not is_admin *);
//!       ------------    --------------    ------------
//!          Check 1         Check 2           Check 1
//!       ------------------------------    ------------
//!                  Branch 1                  Branch 2
//!       ----------------------------------------------
//!                            Tree
//! ```
//!
//! If the block has at least one branch where all checks are truthy, it will pass.
#![doc(html_logo_url = "https://raw.githubusercontent.com/jmkng/ban/main/public/ban.svg")]
#![deny(unsafe_code)]
#![warn(clippy::missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

pub mod filter;

mod compile;
mod engine;
mod log;
mod pipe;
mod region;
mod render;
mod store;
mod syntax;

pub use compile::{compile, Template};
pub use engine::Engine;
pub use render::render;
pub use store::Store;
pub use syntax::Builder;

use syntax::Marker;

/// Create a new instance of `Engine` using the default `Syntax`.
///
/// Equivalent to [`default`][`crate::Engine::default()`]
///
/// # Examples
///
/// ```
/// let engine = ban::default();
/// ```
pub fn default<'source>() -> Engine<'source> {
    Engine::default()
}
