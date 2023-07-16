//! Minimal and fast template engine.
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
pub use serde_json::Value;
pub use store::Store;
pub use syntax::Builder;

pub(crate) use compile::{Parser, Scope, Template};
pub(crate) use pipe::Pipe;
pub(crate) use render::Renderer;

use engine::Engine;
use syntax::Marker;

/// Create a new instance of Ash.
pub fn new<'source>() -> Engine<'source> {
    Engine::default()
}
