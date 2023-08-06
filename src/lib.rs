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
//! Create a new [`Engine`][`crate::Engine`] with [`default`][`crate::default`],
//! or if you want to use custom delimiters, use the [`Builder`][`crate::Builder`]
//! type and the [`new`][`crate::new`] method.
//!
//! ```
//! let engine = ban::default();
//! ```
//!
//! The [`Engine`] type provides a place for you to register
//! [`filters`][`crate::filter::Filter`] and store other
//! [`templates`][`crate::Template`] that you can call with the `include` block.
//!
//! ## Compile
//!
//! Use the [`Engine`] to compile a [`Template`].
//!
//! ```
//! let engine = ban::default();
//! let template = engine.compile("hello (( name ))!");
//! ```
//!
//! ## Create a Store
//!
//! The [`Template`] we just compiled has a single expression that wants to
//! render something called "name".
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
//! Now that we have a [`Store`] containing the data our [`Template`] wants
//! to use, we can tell the [`Engine`] to render it.
//!
//! ```
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
//! # Syntax
//!
//! Ban doesn't have a lot of complicated syntax to learn. It should be
//! familiar if you've used any other template engine.
//!
//! This section provides an overview of expressions and the different
//! blocks you can use.
//!
//! ## Expressions
//!
//! Expressions let you render content from the [`Store`], or literal values
//! like strings and numbers. They look like this:
//!
//! ```text
//! (( name ))
//! ```
//!
//! Or, if you want to transform the "name" variable using filters:
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
//! the final filter in the chain is what gets rendered.
//!
//! Filters may accept any number of arguments, or none. They may be named
//! or anonymous.
//!
//! Named arguments look like this:
//!
//! ```text
//! (( name | tag name: "taylor" age: 25 ))
//! ```
//!
//! Anonymous arguments work the same way, but have no explicit name:
//!
//! ```text
//! (( name | tag "taylor" 25 ))
//! ```
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
//! (* if not false && 500 > 10 *)
//!     hello
//! (* end *)
//! ```
//!
//! You can view an if block as a tree, which may have one or more branches.
//! Each branch may also have one or more leaves.
//!
//! The `||` and `&&` operators are used to divide branches and leaves respectively,
//! similar to how they are used in many programming languages.
//!
//! For an if block to execute, it must have at least one branch where all leaves
//! are truthy.
//!
//! ```text
//!                                        |---| negated
//! (* if this >= that && these == those || not is_admin *);
//!       ------------    --------------    ------------
//!          Leaf 1           Leaf 2            Leaf 1
//!       ------------------------------    ------------
//!                   Branch 1                 Branch 2
//!       ----------------------------------------------
//!                            Tree
//! ```
//!
//! ### Examples
//!
//! ```
//! use ban::{Store};
//! let mut engine = ban::default();
//!
//! let template = engine.compile("(* if first > second *)hello(* else *)goodbye(* end *)");
//! let store = Store::new()
//!     .with_must("first", 100)
//!     .with_must("second", 10);
//!
//! let result = engine.render(&template.unwrap(), &store);
//! assert_eq!(result.unwrap(), "hello");
//!```
//!
//! ## For
//!
//! For blocks allow iteration over a value.
//!
//! You can iterate over arrays, objects, and strings.
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
//! The values held by the identifiers depends on the type you are iterating
//! on:
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
//! let mut engine = ban::default();
//!
//! let template = engine.compile("(* for item in inventory *)(( item )), (* end *)");
//! let store = Store::new()
//!     .with_must("inventory", json!(["sword", "shield"]));
//!
//! let result = engine.render(&template.unwrap(), &store);
//! assert_eq!(result.unwrap(), "sword, shield, ");
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
//! The left side of the expression must be an identifier, meaning an unquoted string,
//! but the right side can be an identifier or literal value.
//!
//! Assignments made within a for block are scoped to the lifetime of that block:
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
//! Assignments made outside of a loop are available globally:
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
//! ### Examples
//!
//! ```
//! use ban::Store;
//!
//! let mut engine = ban::default();
//!
//! let template = engine.compile("hello, (* let name = \"taylor\" -*) (( name ))!");
//! let result = engine.render(&template.unwrap(), &Store::new());
//! assert_eq!(result.unwrap(), "hello, taylor!");
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
//! If you call another template as seen above, it will have access to the same store
//! as the template that called it.
//!
//! You can pass arguments, similar to filters:
//!
//! ```text
//! (* include header name: data.name *)
//! ```
//!
//! When you pass arguments to an included template, it will have access to those values
//! and nothing else.
//!
//! ### Examples
//!
//! ```rust
//! use ban::{filter::serde::json, Store};
//!
//! let mut engine = ban::default();
//! engine
//!     .add_template_must("header", "hello, (( name ))!")
//!     .unwrap();
//!
//! let template = engine.compile(r#"(* include header name: data.name *)"#).unwrap();
//!
//! let result = engine.render(
//!     &template,
//!     &Store::new().with_must("data", json!({"name": "taylor", "age": 25})),
//! );
//! assert_eq!(result.unwrap(), "hello, taylor!");
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
//! A template extends another template when the "extends" expression is found first in the
//! template source.
//!
//! When Ban renders an extended template, all of the block declarations in the source are
//! collected and carried to the parent. Assuming the parent template is not also extended,
//! the blocks are rendered there.
//!
//! When a parent (non-extended) template is rendered and a block expression is encountered,
//! ban will check to see if it has a block from a child template with the same name. If it
//! does, the block is rendered.
//!
//! When no matching block is found, any data inside of the block is rendered instead as a
//! default value.
//!
//! ```rust
//! use ban::{Engine, Store};
//!
//! let mut engine = Engine::default();
//! engine
//!     .add_template_must(
//!         "first", //                                           ----------- default
//!         "hello (* block name *)(* end *), (* block greeting *)doing well?(* end *)",
//!     )
//!     .unwrap();
//! engine
//!     .add_template_must(
//!         "second",
//!         "(* extends first *)\
//!         (* block name *)\
//!         (( name ))\
//!         (* end *)",
//!     )
//!     .unwrap();
//! let store = Store::new().with_must("name", "taylor");
//! let template = engine.get_template("second").unwrap();
//!
//! assert_eq!(
//!     engine.render(&template, &store).unwrap(),
//!     "hello taylor, doing well?"
//! );
//!```
//!
//! View the `examples/inheritance` directory for a full illustration.
//!
//! ## Delimiters
//!
//! Use the `Builder` type to enable custom delimiters.
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
//! let template = engine
//!     .compile("{@ if true ~@}     Hello, >> name <<!{@ end @}")
//!     .unwrap();
//! let result = engine.render(&template, &Store::new().with_must("name", "taylor"));
//!
//! assert_eq!(result, Ok("Hello, taylor!".into()))
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
