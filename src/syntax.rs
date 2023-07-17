//! Ash uses a type known as a Syntax to understand what delimiters
//! you would like to use in your templates. This module defines the
//! Builder type, which provides methods to easily generate a `Syntax`.
//!
//! After a Syntax has been created, it can be passed to an Engine
//! and used to compile templates.
mod builder;

pub use builder::{Builder, Marker};
