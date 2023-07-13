//! Ash - Template Engine
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

use engine::Engine;
use syntax::Marker;

/// Create a new instance of Ash.
pub fn new<'source>() -> Engine<'source> {
    Engine::default()
}
