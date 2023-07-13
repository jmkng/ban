//! Contains the Ash type, which is the main entry point.
use crate::{
    compile::{Parser, Template},
    context::Context,
    general_error,
    render::Renderer,
    Error, Filter,
};
use std::collections::HashMap;

/// Ash entry point.
///
/// Allows registering filters, compiling Template instances from strings,
/// and rendering Template instances with some context data.
pub struct Engine<'source> {
    /// Filters that this engine is aware of.
    filters: HashMap<String, Box<dyn Filter>>,
    /// Templates that this Engine is aware of.
    templates: HashMap<String, Template<'source>>,
}

/// Register a filter with Tera.
///
/// If a filter with that name already exists, it will be overwritten
///
/// ```no_compile
/// tera.register_filter("upper", string::upper);
/// ```
// pub fn register_filter<F: Filter + 'static>(&mut self, name: &str, filter: F) {
//     s

impl<'source> Engine<'source> {
    /// Compile a new Template.
    #[inline]
    pub fn compile(&self, text: &'source str) -> Result<Template<'source>, Error> {
        Parser::new(text).compile()
    }

    /// Render a Template against the given Context.
    #[inline]
    pub fn render(&self, template: Template, context: &Context) -> Result<String, Error> {
        Renderer::new(self, template, context).render()
    }

    /// Add a Filter.
    ///
    /// # Errors
    ///
    /// If a Filter with the given name already exists in the engine, an error is returned.
    pub fn add_filter<T>(&mut self, name: &str, filter: T) -> Result<(), Error>
    where
        T: Filter + 'static,
    {
        let as_string = name.to_string();
        if self.filters.get(&as_string).is_some() {
            return general_error!("filter with name {name} already exists in engine");
        }
        self.filters.insert(as_string, Box::new(filter));
        Ok(())
    }

    /// Add a Filter.
    ///
    /// If a Filter with the given name already exists in the engine, it is overwritten.
    #[inline]
    pub fn add_filter_must<T>(&mut self, name: &str, filter: T)
    where
        T: Filter + 'static,
    {
        self.filters.insert(name.to_string(), Box::new(filter));
    }

    /// Return the filter with the given name, if it exists in Engine.
    #[inline]
    pub fn get_filter(&self, name: &'source str) -> Option<&Box<dyn Filter>> {
        self.filters.get(name)
    }
}

impl<'source> Default for Engine<'source> {
    fn default() -> Self {
        Self {
            filters: HashMap::new(),
            templates: HashMap::new(),
        }
    }
}
