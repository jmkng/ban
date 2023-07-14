mod renderer;

pub use renderer::Renderer;

use crate::{Context, Engine, Error, Template};

/// Render a template.
///
/// Provides a shortcut to quickly render a Template, if no advanced features
/// are needed.
///
/// You may also prefer to create an Engine instance with [ash::new()] if you
/// would like to make custom filter functions available to your template.
pub fn render(template: Template, context: &Context) -> Result<String, Error> {
    Renderer::new(&Engine::default(), template, context).render()
}
