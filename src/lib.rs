//! Ash - Template Engine
mod compile;
mod error;
mod region;
mod syntax;

pub use compile::compile;
pub use syntax::{Marker, SyntaxBuilder};
