use crate::{log::INVALID_FILTER, Error, Filter, Parser, Renderer, Store, Template};
use std::collections::HashMap;

/// Ash entry point.
///
/// Allows registering filters, compiling Template instances from strings,
/// and rendering Template instances with some Store data.
pub struct Engine<'source> {
    /// Filters that this engine is aware of.
    filters: HashMap<String, Box<dyn Filter>>,
    /// Templates that this Engine is aware of.
    templates: HashMap<String, Template<'source>>,
}

impl<'source> Engine<'source> {
    /// Compile a new Template.
    #[inline]
    pub fn compile(&self, text: &'source str) -> Result<Template<'source>, Error> {
        Parser::new(text).compile()
    }

    /// Render a Template with the given Store.
    #[inline]
    pub fn render(&self, template: Template, store: &Store) -> Result<String, Error> {
        Renderer::new(self, template, store).render()
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
            return Err(Error::build(INVALID_FILTER).help(format!("filter with name `{name}` already exists in engine, overwrite it with `.add_filter_must`")));
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