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
    /// Create a new instance of [`Engine`] with the given `Syntax`.
    ///
    /// Note: This method is a stub and is not yet implemented.
    #[inline]
    pub fn new() -> Self {
        todo!()
        // This function will accept and use a Syntax to parse templates,
        // but isn't implemented yet.
    }

    /// Compile a new [`Template`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when compilation fails, which most likely means the source
    /// contains invalid syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let engine = Engine::default();
    /// let template = engine.compile("hello, (( name ))!");
    /// assert!(template.is_ok());
    /// ```
    #[inline]
    pub fn compile(&self, text: &'source str) -> Result<Template<'source>, Error> {
        Parser::new(text).compile(None)
    }

    /// Compile a new [`Template`].
    ///
    /// # Panics
    ///
    /// Panics when compilation fails, which most likely means the source
    /// contains invalid syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let engine = Engine::default();
    /// let template = engine.compile_must("hello, (( name ))!");
    /// ```
    #[inline]
    pub fn compile_must(&self, text: &'source str) -> Template<'source> {
        self.compile(text).unwrap()
    }

    /// Render a [`Template`] with the given [`Store`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering fails, which may happen when a [`Filter`] returns
    /// an `Error` itself, or the template cannot be rendered for a reason that will
    /// be described by the `Error`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{Store, Engine};
    ///
    /// let engine = Engine::default();
    /// let template = engine.compile_must("hello, (( name ))!");
    /// let result = engine.render(&template, &Store::new().with_must("name", "taylor"));
    ///
    /// assert_eq!(result.unwrap(), "hello, taylor!")
    /// ```
    #[inline]
    pub fn render(&self, template: &'source Template, store: &Store) -> Result<String, Error> {
        Renderer::new(self, template, store).render()
    }

    ///
    pub fn render_named(&self, name: &str, store: &Store) -> Result<String, Error> {
        let template = self.get_template(name);
        if template.is_none() {
            return Err(Error::build(format!(
                "template with name `{name}` not found in engine, \
                add it with `.add_template"
            )));
        }
        self.render(template.unwrap(), store)
    }

    /// Compile and store a new [`Template`] with the given name.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when a `Template` with the given name already exists,
    /// or when compilation fails, which most likely means the source contains invalid
    /// syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let mut engine = Engine::default();
    /// let result = engine.add_template("template_name", "hello, (( name ))!");
    /// assert!(result.is_ok());
    ///
    /// let second = engine.add_template("template_name", "hello again");
    /// assert!(second.is_err());
    /// ```
    pub fn add_template(&mut self, name: &'source str, text: &'source str) -> Result<(), Error> {
        if let Some(_) = self.templates.get(name) {
            return Err(Error::build(format!(
                "template with name `{name}` already exists in engine, \
                overwrite it with `.add_template_must"
            )));
        }

        let template = Parser::new(text)
            .compile(Some(name))
            .map_err(|e| e.template(name))?;

        self.templates.insert(name.to_owned(), template);
        Ok(())
    }

    /// Compile and store a new [`Template`] with the given name.
    ///
    /// If a `Template` with the given name already exists in the [`Engine`],
    /// it is overwritten.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when compilation fails, which most likely means the source
    /// contains invalid syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let mut engine = Engine::default();
    /// engine.add_template_must("template_name", "hello, (( name ))!");
    /// ```
    pub fn add_template_must(
        &mut self,
        name: &'source str,
        text: &'source str,
    ) -> Result<(), Error> {
        let template = Parser::new(text)
            .compile(Some(name))
            .map_err(|e| e.template(name))?;

        self.templates.insert(name.to_owned(), template);
        Ok(())
    }

    /// Return the named [`Template`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let mut engine = Engine::default();
    /// engine.add_template_must("template_name", "hello, (( name ))!");
    ///
    /// let template = engine.get_template("template_name");
    /// assert!(template.is_some());
    pub fn get_template(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    /// Add a [`Filter`].
    ///
    /// # Errors
    ///
    /// If a `Filter` with the given name already exists in the engine, an [`Error`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine, Store,
    /// };
    /// use std::collections::HashMap;
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .help("use quotes to coerce data to string")
    ///         ),
    ///     }
    /// };
    ///
    /// let mut engine = Engine::default();
    /// let result = engine.add_filter("to_lowercase", to_lowercase);
    ///
    /// assert!(result.is_ok());
    /// ```
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

    /// Add a [`Filter`].
    ///
    /// If a `Filter` with the given name already exists in the [`Engine`], it is overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine, Store,
    /// };
    /// use std::collections::HashMap;
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .help("use quotes to coerce data to string")
    ///         ),
    ///     }
    /// };
    ///
    /// let mut engine = Engine::default();
    /// let result = engine.add_filter_must("to_lowercase", to_lowercase);
    /// ```
    #[inline]
    pub fn add_filter_must<T>(&mut self, name: &str, filter: T)
    where
        T: Filter + 'static,
    {
        self.filters.insert(name.to_string(), Box::new(filter));
    }

    /// Add a [`Filter`].
    ///
    /// Returns the [`Engine`], so additional methods may be chained.
    ///
    /// # Errors
    ///
    /// If a `Filter` with the given name already exists in the engine, an [`Error`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine, Store,
    /// };
    /// use std::collections::HashMap;
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .help("use quotes to coerce data to string")
    ///         ),
    ///     }
    /// };
    ///
    /// let result = Engine::default().with_filter("to_lowercase", to_lowercase);
    ///
    /// assert!(result.is_ok());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine, Store,
    /// };
    /// use std::collections::HashMap;
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .help("use quotes to coerce data to string")
    ///         ),
    ///     }
    /// };
    ///
    /// let engine = Engine::default().with_filter_must("to_lowercase", to_lowercase);
    /// ```
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
