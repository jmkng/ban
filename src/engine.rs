use crate::{
    compile::{Parser, Template},
    filter::Filter,
    log::Error,
    log::INVALID_FILTER,
    render::Renderer,
    Store,
};
use std::collections::HashMap;

/// Facilitates compiling and rendering templates, and provides storage
/// for filters.
pub struct Engine<'source> {
    /// Filters that this engine is aware of.
    filters: HashMap<String, Box<dyn Filter>>,
    /// Templates that this Engine is aware of.
    templates: HashMap<String, Template<'source>>,
}

impl<'source> Engine<'source> {
    /// Create a new instance of Engine with the given Syntax.
    #[inline]
    pub fn new() -> Self {
        todo!()
        // This function will accept and use a Syntax to parse templates,
        // but isn't implemented yet.
    }

    /// Compile a new Template.
    ///
    /// # Errors
    ///
    /// Returns an Error when compilation fails, which most likely means the source
    /// contains invalid syntax.
    #[inline]
    pub fn compile(&self, text: &'source str) -> Result<Template<'source>, Error> {
        Parser::new(text).compile()
    }

    /// Compile a new Template.
    ///
    /// # Panics
    ///
    /// Panics when compilation fails, which most likely means the source
    /// contains invalid syntax.
    #[inline]
    pub fn compile_must(&self, text: &'source str) -> Template<'source> {
        self.compile(text).unwrap()
    }

    /// Render a Template with the given Store.
    #[inline]
    pub fn render(&self, template: &'source Template, store: &Store) -> Result<String, Error> {
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
            return Err(Error::build(INVALID_FILTER).help(format!(
                "filter with name `{name}` already exists in engine, \
                overwrite it with `.add_filter_must`"
            )));
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

    /// Add a Filter.
    ///
    /// Returns the Engine, so additional methods may be chained.
    ///
    /// # Errors
    ///
    /// If a Filter with the given name already exists in the engine, an error is returned.
    #[inline]
    pub fn with_filter<T>(mut self, name: &str, filter: T) -> Result<Self, Error>
    where
        T: Filter + 'static,
    {
        self.add_filter(name, filter)?;
        Ok(self)
    }

    /// Add a Filter.
    ///
    /// Returns the Engine, so additional methods may be chained.
    ///
    /// If a Filter with the given name already exists in the engine, it is overwritten.
    #[inline]
    pub fn with_filter_must<T>(mut self, name: &str, filter: T) -> Self
    where
        T: Filter + 'static,
    {
        self.add_filter_must(name, filter);
        self
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

#[cfg(test)]
mod tests {
    use crate::{engine::Engine, log::Error};
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_add() {
        let mut engine = Engine::default();
        engine.add_filter_must("faux", faux_filter_a);

        assert!(engine.get_filter("faux").is_some());
        assert!(engine.get_filter("ghost").is_none())
    }

    #[test]
    fn test_add_fluent() {
        assert!(Engine::default()
            .with_filter("faux", faux_filter_a)
            .unwrap()
            .get_filter("faux")
            .is_some());
        assert!(Engine::default().get_filter("ghost").is_none());
    }

    #[test]
    fn test_add_duplicate() {
        assert!(Engine::default()
            .with_filter_must("faux", faux_filter_a)
            .with_filter("faux", faux_filter_a)
            .is_err())
    }

    #[test]
    fn test_add_overwrite() {
        let value = Value::Null;
        let arguments = HashMap::new();

        let mut engine = Engine::default().with_filter_must("faux", faux_filter_a);
        assert!(engine.get_filter("faux").is_some_and(|f| f
            .apply(&value, &arguments)
            .is_ok_and(|v| v == Value::String("a".into()))));

        engine.add_filter_must("faux", faux_filter_b);
        assert!(engine.get_filter("faux").is_some_and(|f| f
            .apply(&value, &arguments)
            .is_ok_and(|v| v == Value::String("b".into()))));
    }

    /// A Filter used to test Engine.
    fn faux_filter_a(_: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        Ok(Value::String("a".into()))
    }

    /// A Filter used to test Engine.
    fn faux_filter_b(_: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        Ok(Value::String("b".into()))
    }
}
