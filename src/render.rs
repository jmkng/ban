mod renderer;

pub use renderer::Renderer;

use crate::{Engine, Error, Store, Template};

/// Render a template.
///
/// Provides a shortcut to quickly render a Template, if no advanced features
/// are needed.
///
/// You may also prefer to create an Engine instance with [ash::new()] if you
/// would like to make custom filter functions available to your template.
pub fn render<'source>(template: &'source Template, store: &Store) -> Result<String, Error> {
    Renderer::new(&Engine::default(), template, store).render()
}
