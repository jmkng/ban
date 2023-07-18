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
///
/// # Examples
///
/// ```
/// use ban::{compile, render, Store};
///
/// let template = compile("hello, (( name ))!");
/// assert!(template.is_ok());
///
/// let output = render(&template.unwrap(), &Store::new().with_must("name", "taylor"));
/// assert_eq!(output.unwrap(), "hello, taylor!");
/// ```
pub fn render<'source>(template: &'source Template, store: &Store) -> Result<String, Error> {
    Renderer::new(&Engine::default(), template, store).render()
}
