mod renderer;

pub use renderer::Renderer;

use crate::{compile::Template, log::Error, Engine, Store};

/// Render a `Template`.
///
/// Provides a shortcut to quickly render a `Template` when no advanced features
/// are needed.
///
/// You may also prefer to create an [`Engine`][`crate::Engine`] if you intend to
/// use custom filters in your templates.
pub fn render<'source>(template: &'source Template, store: &Store) -> Result<String, Error> {
    Renderer::new(&Engine::default(), template, store).render()
}
