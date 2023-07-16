//! Minimal and fast template engine.
#![deny(unsafe_code)]
#![warn(clippy::missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

pub mod serde_json {
    pub use serde_json::*;
}

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
pub(crate) use compile::{Parser, Scope, Template};
pub use filter::Filter;
pub use log::{Error, Pointer, Visual};
pub(crate) use pipe::Pipe;
pub use region::Region;
pub use render::render;
pub(crate) use render::Renderer;
pub use store::Store;
pub use syntax::Builder;

use engine::Engine;
use syntax::Marker;

/// Create a new instance of Ash.
pub fn new<'source>() -> Engine<'source> {
    Engine::default()
}
