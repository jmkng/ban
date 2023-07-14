//! Ash - Template Engine

#![deny(unsafe_code)]
#![warn(clippy::missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod compile;
mod context;
mod engine;
mod error;
mod filter;
mod format;
mod region;
mod render;
mod syntax;

pub use compile::compile;
pub use context::Context;
pub use error::Error;
pub use filter::Filter;
pub use render::render;
pub use serde_json::Value;
pub use syntax::Builder;

pub(crate) use compile::{Parser, Scope, Template};
pub(crate) use format::Formatter;
pub(crate) use region::Region;
pub(crate) use render::Renderer;

use engine::Engine;
use syntax::Marker;

/// Create a new instance of Ash.
pub fn new<'source>() -> Engine<'source> {
    Engine::default()
}
