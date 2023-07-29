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
//!
//! ## For
//!
//! For blocks allow you to iterate over data.
//!
//! ```html
//! (* for item in inventory *)
//!     (( item ))
//! (* endfor *)
//! ```
//!
//! You can iterate over arrays, objects and strings.
//!
//! Providing a single identifier before the "in" keyword will scope the value to the
//! identifier, but you can also provide two identifiers separated by a comma:
//!
//! ```html
//! (* for i, item in inventory *)
//!     (( i )) - (( item ))
//! (* endfor *)
//! ```
//!
//! The value of the two identifiers will vary based on the type you are iterating on.
//! Considering the above example, these rules apply:
//!
//! When "inventory" is..
//!
//! 1. Object:
//!     - i: key
//!     - item: value
//! 2. Array:
//!     - i: index
//!     - item: value
//! 3. String:
//!     - i: index
//!     - item: char
//!
//! ## Let
//!
//! Let blocks allow you to assign values to identifiers.
//!
//!
//! ```html
//! (* let name = "taylor" *)
//! ```
//!
//! The left side of the expression expects an identifier, meaning an unquoted string,
//! while the right side may be an identifier pointing to some value within the [`Store`],
//! or literal data as seen in the example above.
//!
//! Assignments made within a for block will be scoped to the lifetime of that for block,
//! while assignments made anywhere else are globally scoped.
//!
//! In this example, the "name" variable exists within the loop, but will not be found
//! after it ends:
//!
//! ```html
//! (* for item in inventory *)
//!     (* let name = item.description.name *)
//!     Item: (( name ))
//! (* endfor *)
//!
//! Last item name: (( name )). // <-- error
//! ```
//!
//! In this example, an if block is used to conditionally assign a variable that is
//! available globally.
//!
//! ```html
//! (* if is_admin *)
//!     (* let name = "admin" *)
//! (* else *)
//!     (* let name = user.name *)
//! (* endif *)
//!
//! Hello, (( name )).
//! ```
//!
//! ## Include
//!
//! Include blocks allow you to render other registered templates.
//!
//! ```html
//! (* include header *)
//! ```
//!
//! In the above example, "header" is the name of a registered template.
//! Quoted and unquoted strings are recognized here, but you will most likely want
//! to use an unquoted string, unless you registered your template with a name that
//! contains quotes.
//!
//! ```rust
//! use ban::Store;
//!
//! let mut engine = ban::default();
//! engine.add_template_must("header", "hello, there!").unwrap();
//!
//! let template = engine.compile(r#"(* include "header" *)"#).unwrap();
//! let result = engine.render(&template, &Store::new());
//! assert!(result.is_err());
//!```
//!
//! The above example produces this error:
//!
//! ```html
//! --> ?:1:12
//!   |
//! 1 | (* include "header" *)
//!   |            ^^^^^^^^
//!   |
//! = help: try adding the template `"header"` to the engine with `.add_template` first,
//! or you may want to remove the surrounding quotes
//! ```
//!
//! To resolve this, you may either register your template with the string `"\"header\""`,
//! or as the error suggests, remove the quotes from the include block.
//!
//! When an include block is used, the called template will have access to the same store
//! data that the calling template does, unless you scope the data by providing arguments.
//! In this example, the "header" template only has access to a "name" variable.
//!
//! ```rust
//! let mut engine = crate::default();
//! engine
//!     .add_template_must("header", "hello, (( name ))!")
//!     .unwrap();
//!
//! let template = engine
//!     .compile(r#"(* include header name: data.name *)"#)
//!     .unwrap();
//!
//! let result = engine.render(
//!     &template,
//!     &Store::new().with_must("data", json!({"name": "taylor", "age": 25})),
//! );
//! assert_eq!(result.unwrap(), "hello, taylor!");
//!```
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
use serde_json::json;
pub use store::Store;
pub use syntax::Builder;

use syntax::Marker;

/// Create a new instance of [`Engine`] using the default `Syntax`.
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
