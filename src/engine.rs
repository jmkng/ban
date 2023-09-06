use std::collections::HashMap;

use crate::{
    compile::{Parser, Template},
    log::Error,
    render::{filter::Filter, pipe::Pipe, Renderer},
    Builder, Store,
};

use morel::{Finder, Syntax};

pub const INVALID_FILTER: &str = "invalid filter";

/// Facilitates compiling and rendering templates, and provides storage
/// for filters.
pub struct Engine {
    /// [`Filter`] instances assigned to this [`Engine`].
    filters: HashMap<String, Box<dyn Filter>>,
    /// [`Template`] instances assigned to this [`Engine`].
    templates: HashMap<String, Template>,
    /// [`Finder`] used to compile [`Template`] instances.
    finder: Finder,
}

impl Engine {
    /// Create a new [`Engine`] with the given `Syntax`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// let syntax = Builder::new()
    ///     .with_expression("{{", "}}")
    ///     .with_block("{*", "*}")
    ///     .to_syntax();
    ///
    /// let engine = ban::new(syntax);
    /// ```
    #[inline]
    pub fn new(syntax: Syntax) -> Self {
        Self {
            filters: HashMap::new(),
            templates: HashMap::new(),
            finder: Finder::new(syntax),
        }
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
    pub fn compile(&self, text: &str) -> Result<Template, Error> {
        Parser::new(text, &self.finder).compile(None)
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
    pub fn compile_must(&self, text: &str) -> Template {
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
    pub fn render(&self, template: &Template, store: &Store) -> Result<String, Error> {
        let mut buffer = get_buffer(template);
        Renderer::new(self, template, store).render(&mut Pipe::new(&mut buffer))?;

        Ok(buffer)
    }

    /// Store an existing [`Template`] in the [`Engine`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let mut first_engine = Engine::default();
    /// let template = first_engine.compile("hello, (( name ))!").unwrap();
    ///
    /// let mut second_engine = Engine::default();
    /// let mut third_engine = Engine::default();
    ///
    /// // If the template was pulled from another engine, it might have a name already.
    /// // Use the same name, or overwrite it.
    /// if template.get_name().is_some() {
    ///     second_engine.add_template(template.get_name().unwrap(), template.clone());
    /// } else {
    ///     second_engine.add_template("new_name", template);
    /// }
    /// ```
    pub fn add_template<T>(&mut self, name: T, template: Template)
    where
        T: Into<String>,
    {
        self.templates.insert(name.into(), template);
    }

    /// Store an existing [`Template`] in the [`Engine`].
    ///
    /// Returns the [`Engine`], so additional methods may be chained.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Engine;
    ///
    /// let mut first_engine = Engine::default();
    /// let template = first_engine.compile("hello, (( name ))!").unwrap();
    ///
    /// let mut second_engine = Engine::default();
    /// let mut third_engine = Engine::default();
    ///
    /// // If the template was pulled from another engine, it might have a name already.
    /// // Use the same name, or overwrite it.
    /// if template.get_name().is_some() {
    ///     second_engine.add_template(template.get_name().unwrap(), template.clone());
    /// } else {
    ///     second_engine.add_template("new_name", template);
    /// }
    /// ```
    pub fn with_template<T>(mut self, name: T, template: Template) -> Self
    where
        T: Into<String>,
    {
        self.add_template(name, template);

        self
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
    /// let result = engine.insert_template("template_name", "hello, (( name ))!");
    /// assert!(result.is_ok());
    ///
    /// let second = engine.insert_template("template_name", "hello again");
    /// assert!(second.is_err());
    /// ```
    pub fn insert_template(&mut self, name: &str, text: &str) -> Result<(), Error> {
        if let Some(_) = self.templates.get(name) {
            return Err(Error::build(format!(
                "template with name `{name}` already exists in engine, \
                overwrite it with `.add_template_must"
            )));
        }

        let template = Parser::new(text, &self.finder)
            .compile(Some(name.to_owned()))
            .map_err(|error| error.with_name(name))?;

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
    /// engine.insert_template_must("template_name", "hello, (( name ))!");
    /// ```
    pub fn insert_template_must(&mut self, name: &str, text: &str) -> Result<(), Error> {
        let template = Parser::new(text, &self.finder)
            .compile(Some(name.to_owned()))
            .map_err(|error| error.with_name(name))?;

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
    /// engine.insert_template_must("template_name", "hello, (( name ))!");
    ///
    /// let template = engine.get_template("template_name");
    /// assert!(template.is_some());
    /// ```
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
    /// use std::collections::HashMap;
    ///
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine
    /// };
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .with_help("use quotes to coerce data to string")
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
            return Err(Error::build(INVALID_FILTER).with_help(format!(
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
    /// use std::collections::HashMap;
    ///
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine
    /// };
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .with_help("use quotes to coerce data to string")
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
    /// use std::collections::HashMap;
    ///
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine
    /// };
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .with_help("use quotes to coerce data to string")
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
    /// use std::collections::HashMap;
    ///
    /// use ban::{
    ///     filter::{
    ///         serde::{json, Value},
    ///         Error,
    ///     },
    ///     Engine
    /// };
    ///
    /// fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    ///     match value {
    ///         Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
    ///         _ => Err(Error::build("filter `to_lowercase` requires string input")
    ///            .with_help("use quotes to coerce data to string")
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
    pub fn get_filter(&self, name: &str) -> Option<&Box<dyn Filter>> {
        self.filters.get(name)
    }
}

impl Default for Engine {
    /// Create a new [`Engine`] with the default `Syntax`.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = ban::default();
    /// ```
    fn default() -> Self {
        Self {
            filters: HashMap::new(),
            templates: HashMap::new(),
            finder: Finder::new(Builder::new().to_syntax()),
        }
    }
}

/// Return a String with capacity to suit the given [`Template`].
///
/// If the `Template` is extended, a buffer with no capacity is returned,
/// otherwise a buffer with a capacity equal to the length of the source
/// is created.
pub fn get_buffer(template: &Template) -> String {
    match template.get_extends() {
        Some(_) => String::new(),
        None => String::with_capacity(template.get_source().len()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{engine::Engine, log::Error, Store};

    use serde_json::Value;

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
    fn test_add_existing() {
        let mut engine = Engine::default();
        engine
            .insert_template_must("template_name", "faux")
            .unwrap();
        let template = engine.get_template("template_name").unwrap();
        let mut second_engine = Engine::default();
        second_engine.add_template(template.get_name().unwrap(), template.clone());

        assert_eq!(
            second_engine
                .render(
                    second_engine.get_template("template_name").unwrap(),
                    &Store::new()
                )
                .unwrap(),
            "faux"
        );
        assert!(engine.get_template("template_name").is_some())
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

    /// A [`Filter`][`crate::filter::Filter`] used to test Engine.
    fn faux_filter_a(_: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        Ok(Value::String("a".into()))
    }

    /// A [`Filter`][`crate::filter::Filter`] used to test Engine.
    fn faux_filter_b(_: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        Ok(Value::String("b".into()))
    }
}
