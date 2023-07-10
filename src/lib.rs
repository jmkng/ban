//! Ash - Template Engine
mod compile;
mod error;
mod filter;
mod region;
mod render;
mod syntax;

pub use compile::compile;
pub(crate) use error::Error;
pub use syntax::{Marker, SyntaxBuilder};
