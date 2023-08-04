mod compare;

use self::compare::{compare_values, is_truthy};
use crate::{
    compile::{
        tree::{
            Arguments, Base, Block, Call, CheckBranch, Expression, Extends, For, Identifier, If,
            Include, Let, Output, Set, Tree,
        },
        Scope, Template,
    },
    engine::get_buffer,
    log::{
        message::{error_missing_template, error_write, INCOMPATIBLE_TYPES, INVALID_FILTER},
        Error,
    },
    pipe::Pipe,
    region::Region,
    store::Shadow,
    Engine, Store,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Write},
    mem::take,
};

/// Render a [`Template`].
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
    let mut buffer = get_buffer(template);
    Renderer::new(&Engine::default(), template, store).render(&mut Pipe::new(&mut buffer))?;

    Ok(buffer)
}

/// Provides methods to render a set of [`Tree`] against some context data.
pub struct Renderer<'source, 'store> {
    /// An [`Engine`] containing any registered filters.
    engine: &'source Engine,
    /// The [`Template`] being rendered.
    template: &'source Template,
    /// Contains the [`Store`] and any shadowed data.
    shadow: Shadow<'store>,
    /// Blocks available for rendering.
    blocks: BlockMap<'source>,
}

impl<'source, 'store> Renderer<'source, 'store> {
    /// Create a new Renderer.
    pub fn new(engine: &'source Engine, template: &'source Template, store: &'store Store) -> Self {
        Renderer {
            engine,
            template,
            shadow: Shadow::new(store),
            blocks: HashMap::new(),
        }
    }

    /// Render the [`Template`] stored inside the [`Renderer`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering any of the [`Tree`] instances within the
    /// `Template` fail to render, or writing a rendered `Tree` to the buffer fails.
    pub fn render(&mut self, pipe: &mut Pipe) -> Result<(), Error> {
        match &self.template.extended {
            Some(extended) => self.evaluate_scope(extended, pipe),
            None => self.render_scope(&self.template.scope, pipe),
        }
        .map_err(|e| {
            if !e.is_named() && self.template.name.is_some() {
                return e.template(self.template.name.as_ref().unwrap());
            }
            e
        })?;
        Ok(())
    }

    /// Evaluate the [`Scope`] and collect all [`Block`] instances within.
    ///
    /// After all `Block` instances are collected, a new Renderer is spawned and used to
    /// render the extended [`Template`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the extended template does not exist, or any [`Tree`]
    /// instances within fail to render.
    fn evaluate_scope(&mut self, extends: &Extends, pipe: &mut Pipe) -> Result<(), Error> {
        let name = extends.name.get_region().literal(&self.template.source);
        let template = self
            .engine
            .get_template(name)
            .ok_or_else(|| error_missing_template(name))?;
        self.collect_blocks(&self.template.scope);

        Renderer::new(self.engine, &template, self.shadow.store)
            .with_blocks(take(&mut self.blocks))
            .render(pipe)
    }

    /// Render a [`Scope`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any of the [`Tree`] instances in the `Scope` cannot
    /// be rendered.
    fn render_scope(&mut self, scope: &'source Scope, pipe: &mut Pipe) -> Result<(), Error> {
        let mut iterator = scope.data.iter();

        while let Some(next) = iterator.next() {
            match next {
                Tree::Raw(ra) => {
                    let value = self.evaluate_raw(ra)?;
                    pipe.write_str(value).map_err(|_| error_write())?
                }
                Tree::Output(ou) => {
                    let value = self.evaluate_output(ou)?;
                    pipe.write_value(&value).map_err(|_| error_write())?
                }
                Tree::If(i) => {
                    self.render_if(i, pipe)?;
                }
                Tree::For(fo) => {
                    self.render_for(fo, pipe)?;
                }
                Tree::Let(le) => {
                    self.evaluate_let(le)?;
                }
                Tree::Include(inc) => {
                    self.render_include(inc, pipe)?;
                }
                Tree::Block(bl) => {
                    self.render_block(bl, pipe)?;
                }
                _ => unreachable!("parser should catch invalid top level tree"),
            }
        }
        Ok(())
    }

    /// Render a [`Block`].
    ///
    /// Searches for and renders a `Block` within the [`Renderer`] that matches the name
    /// of the given `Block`, or when no matching block exists, renders any values within
    /// the `Block` itself as a default.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any of the [`Tree`] instances within the `Block` fail to
    /// render.
    fn render_block(&mut self, block: &'source Block, pipe: &mut Pipe) -> Result<(), Error> {
        let name = block.name.get_region().literal(&self.template.source);

        match self.blocks.get(name) {
            Some(shadowed) => Renderer::new(self.engine, shadowed.template, self.shadow.store)
                .render_scope(&shadowed.block.scope, pipe),
            None => self.render_scope(&block.scope, pipe),
        }
    }

    /// Render an [`Include`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the named [`Template`] is not found in the [`Engine`],
    /// or rendering any [`Tree`] instances within the template fails.
    fn render_include(&mut self, include: &Include, pipe: &mut Pipe) -> Result<(), Error> {
        let name = include.name.get_region().literal(&self.template.source);
        let template = self
            .engine
            .get_template(name)
            .ok_or_else(|| error_missing_template(name))?;

        if include.mount.is_some() {
            // Scoped include, create a new store that includes only the named values.
            let mut scoped_store = Store::new();

            for point in include.mount.as_ref().unwrap().values.iter() {
                let name = point.name.literal(&self.template.source);
                let value = self.evaluate_base(&point.value)?;
                scoped_store.insert_must(name, value);
            }
            Renderer::new(self.engine, template, &scoped_store).render(pipe)?
        } else {
            // Unscoped include, use the same store.
            Renderer::new(self.engine, template, self.shadow.store).render(pipe)?
        };
        Ok(())
    }

    /// Render an [`If`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a [`Scope`] is chosen to be rendered, but a [`Tree`]
    /// instance within the `Scope` fails to render.
    fn render_if(&mut self, i: &'source If, pipe: &mut Pipe) -> Result<(), Error> {
        for branch in i.tree.branches.iter() {
            if !self.evaluate_branch(branch)? {
                if i.else_branch.is_some() {
                    self.render_scope(i.else_branch.as_ref().unwrap(), pipe)?;
                }
                return Ok(());
            }
        }
        self.render_scope(&i.then_branch, pipe)
    }

    /// Render an [`Iterable`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when rendering a [`Tree`] within the [`Scope`] fails,
    /// or the [`Base`] is not found in the [`Store`].
    fn render_for(&mut self, fo: &'source For, pipe: &mut Pipe) -> Result<(), Error> {
        self.shadow.push();

        let value = self.evaluate_base(&fo.base)?;
        match value.as_ref() {
            Value::String(st) => {
                for (index, char) in st.to_owned().char_indices() {
                    self.shadow_set(&fo.set, (Some(index), char))?;
                    self.render_scope(&fo.scope, pipe)?;
                }
            }
            Value::Array(ar) => {
                for (index, value) in ar.to_owned().iter().enumerate() {
                    self.shadow_set(&fo.set, (Some(index), value))?;
                    self.render_scope(&fo.scope, pipe)?;
                }
            }
            Value::Object(ob) => {
                for (key, value) in ob.to_owned().iter() {
                    self.shadow_set(&fo.set, (Some(key), value))?;
                    self.render_scope(&fo.scope, pipe)?;
                }
            }
            incompatible => {
                return Err(Error::build(INCOMPATIBLE_TYPES).help(format!(
                    "iterating on value `{}` is not supported",
                    incompatible
                )))
            }
        }
        self.shadow.pop();

        Ok(())
    }

    /// Evaluate the [`CheckBranch`] and return true if every [`Check`]
    /// within is truthy.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if evaluating a [`Base`] fails, which may happen
    /// when accessing the literal value of a [`Region`] fails.
    fn evaluate_branch(&self, branch: &CheckBranch) -> Result<bool, Error> {
        for check in branch {
            let left = self.evaluate_base(&check.left)?;

            match &check.right {
                Some(base) => {
                    let right = self.evaluate_base(&base)?;
                    let operator = check
                        .operator
                        .expect("if check.right is some, operator must exist");

                    if !compare_values(&left, operator, &right)
                        .map(|b| if check.negate { !b } else { b })
                        .map_err(|e| {
                            e.pointer(
                                &self.template.source,
                                check.left.get_region().combine(base.get_region()),
                            )
                        })?
                    {
                        return Ok(false);
                    }
                }
                None => {
                    let result = match check.negate {
                        true => !is_truthy(&left),
                        false => is_truthy(&left),
                    };

                    if !result {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Evaluate an [`Output`] to return a [`Value`].
    ///
    /// # Errors
    ///
    /// Returns an error if rendering the `Output` fails.
    fn evaluate_output(&self, output: &'source Output) -> Result<Cow<Value>, Error> {
        match &output.expression {
            Expression::Base(base) => self.evaluate_base(base),
            Expression::Call(call) => self.evaluate_call(call),
        }
    }

    /// Evaluate a [`Base`] to return a [`Value`].
    ///
    /// # Errors
    ///
    /// Returns an error if evaluating the `Base` fails, which may happen when
    /// accessing the literal value of a [`Region`] fails.
    fn evaluate_base(&self, base: &'source Base) -> Result<Cow<Value>, Error> {
        match base {
            Base::Variable(variable) => self.evaluate_keys(&variable.path),
            Base::Literal(literal) => Ok(Cow::Borrowed(&literal.value)),
        }
    }

    /// Evaluate a [`Call`] to return a [`Value`].
    ///
    /// Follows the receiver until a [`Base`] is reached, the beginning input
    /// is derived from this base.
    ///
    /// From there, we work in the opposite direction, calling each filter
    /// function one by one until we get back to the end of the `Call`.
    ///
    /// The output of the final `Call` in the chain is the return value.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when rendering the `Base` of the `Call` chain fails,
    /// or executing a [`Filter`][`crate::filter::Filter`] returns an [`Error`] itself.
    fn evaluate_call(&self, call: &'store Call) -> Result<Cow<Value>, Error> {
        let mut call_stack = vec![call];

        let mut receiver: &Expression = &call.receiver;
        while let Expression::Call(call) = receiver {
            call_stack.push(call);
            receiver = &call.receiver;
        }
        let mut value = match receiver {
            Expression::Base(base) => self.evaluate_base(base)?,
            _ => unreachable!(),
        };

        for call in call_stack.iter().rev() {
            let name_literal = call.name.region.literal(&self.template.source);
            let func = self.engine.get_filter(name_literal);
            if func.is_none() {
                return Err(Error::build(INVALID_FILTER)
                    .pointer(&self.template.source, call.name.region)
                    .help(format!(
                        "template wants to use the `{name_literal}` filter, but a filter with that \
                        name was not found in this engine, did you add the filter to the engine with \
                        `.add_filter` or `.add_filter_must`?"
                    )));
            }

            let arguments = if call.arguments.is_some() {
                self.evaluate_arguments(call.arguments.as_ref().unwrap())?
            } else {
                HashMap::new()
            };

            let returned = func
                .unwrap()
                .apply(&value, &arguments)
                .or_else(|e| Err(e.pointer(&self.template.source, call.name.region)))?;

            value = Cow::Owned(returned);
        }

        Ok(value)
    }

    /// Evaluate a [`Region`] to return a &str.
    ///
    /// The literal value of the `Region` within the source text is retrieved
    /// and returned.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if accessing the literal value of the `Region` fails.
    fn evaluate_raw(&self, region: &Region) -> Result<&str, Error> {
        Ok(region.literal(&self.template.source))
    }

    /// Evaluate a set of [`Key`] instances to return a [`Value`] from the [`Store`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when accessing the literal value of the [`Region`]
    /// from any of the `Key` instances fails.
    fn evaluate_keys(&self, keys: &Vec<Identifier>) -> Result<Cow<Value>, Error> {
        let first_region = keys
            .first()
            .expect("key vector should always have at least one key")
            .region;

        let first_value = first_region.literal(&self.template.source);
        let store_value = self.shadow.get(first_value);

        let mut value: Cow<Value> = if store_value.is_some() {
            Cow::Borrowed(store_value.unwrap())
        } else {
            return Err(Error::build("missing store value")
                .pointer(&self.template.source, first_region)
                .help(format!(
                    "unable to find `{first_value}` in store, \
                    ensure it exists or try wrapping with an `if` block",
                )));
        };

        for key in keys.iter().skip(1) {
            match value.as_object() {
                Some(object) => {
                    let key_region = key.region;
                    let key_name = key_region.literal(&self.template.source);
                    let next_object = object.get(key_name);

                    value = if next_object.is_some() {
                        Cow::Owned(next_object.unwrap().clone())
                    } else {
                        Cow::Owned(Value::Null)
                    };
                }
                None => return Ok(Cow::Owned(Value::Null)),
            }
        }

        Ok(value)
    }

    /// Evaluate an [`Arguments`] to return a [`HashMap`] that contains the same values.
    ///
    /// As described in the filter module, any argument without a name will
    /// be automatically assigned a name.
    ///
    /// # Errors
    ///
    /// Propagates an [`Error`] if rendering a [`Base`] fails, which may happen when the literal
    /// value of a [`Region`] cannot be accessed.
    fn evaluate_arguments(&self, arguments: &Arguments) -> Result<HashMap<String, Value>, Error> {
        let mut buffer = HashMap::new();
        let mut unnamed = 1;

        for arg in &arguments.values {
            let name = if arg.name.is_some() {
                arg.name.unwrap().literal(&self.template.source).to_string()
            } else {
                let temp = unnamed;
                unnamed += 1;
                temp.to_string()
            };

            let value = self.evaluate_base(&arg.value)?;
            buffer.insert(name, value.into_owned());
        }

        Ok(buffer)
    }

    /// Evaluate a [`Let`] to make an assignment to the current [`Shadow`] scope.
    fn evaluate_let(&mut self, le: &Let) -> Result<(), Error> {
        let value = self.evaluate_base(&le.right)?;
        self.shadow_set(
            &Set::Single(le.left.clone()),
            (None::<Value>, value.into_owned()),
        )?;
        Ok(())
    }

    fn with_blocks(mut self, blocks: BlockMap<'source>) -> Self {
        self.blocks = blocks;
        self
    }

    /// Clone all of the [`Block`] instances in the given [`Scope`] into
    /// the Renderer.
    fn collect_blocks(&mut self, scope: &'source Scope) {
        let mut iterator = scope.data.iter();

        while let Some(next) = iterator.next() {
            match next {
                Tree::Block(block) => {
                    let name = block.name.get_region().literal(&self.template.source);
                    self.blocks.insert(
                        name.to_string(),
                        Named {
                            template: self.template,
                            block: block.clone(),
                        },
                    );
                }
                // In an extended template, all non-blocks are ignored.
                _ => {}
            }
        }
    }

    /// Assign the given data to the [`Set`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when accessing the literal value of a [`Region`] fails.
    ///
    /// # Panics
    ///
    /// Panics when a `Set` of type `Pair` is received, but the .0 property in the
    /// "pair" parameter is None.
    fn shadow_set<N, T>(&mut self, set: &Set, data: (Option<N>, T)) -> Result<(), Error>
    where
        N: Serialize + Display,
        T: Serialize + Display,
    {
        let source = &self.template.source;
        match set {
            Set::Single(si) => {
                let key = si.region.literal(&source);
                self.shadow.insert_must(key, data.1)
            }
            Set::Pair(pa) => {
                let key = pa.key.region.literal(&source);
                let value = pa.value.region.literal(&source);
                self.shadow.insert_must(key, data.0.unwrap());
                self.shadow.insert_must(value, data.1);
            }
        }
        Ok(())
    }
}

type BlockMap<'source> = HashMap<String, Named<'source>>;

/// A wrapper for [`Block`] that includes a reference to the [`Template`]
/// that the `Block` was found in.
struct Named<'source> {
    /// The [`Template`] that the [`Block`] was found in.
    template: &'source Template,
    /// The [`Block`] found in [`Template`].
    block: Block,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{Engine, Store, Template};

    #[test]
    fn test_render_raw() {
        let (template, engine) = get_template_with_engine("hello there");

        assert_eq!(
            engine.render(&template, &Store::new()).unwrap(),
            "hello there"
        );
    }

    #[test]
    fn test_render_output() {
        let (template, engine) = get_template_with_engine("hello there, (( name ))!");
        let store = Store::new().with_must("name", "taylor");

        assert_eq!(
            engine.render(&template, &store).unwrap(),
            "hello there, taylor!"
        );
    }

    #[test]
    fn test_render_output_whitespace() {
        let (template, engine) = get_template_with_engine("hello there, ((- name -)) !");
        let store = Store::new().with_must("name", "taylor");

        assert_eq!(
            engine.render(&template, &store).unwrap(),
            "hello there,taylor!"
        );
    }

    #[test]
    fn test_render_if() {
        let (template, engine) = get_template_with_engine(
            "(* if left > 300 *)\
                a\
            (* else if name == \"taylor\" *)\
                b\
            (* else if not false *)\
                c\
            (* else *)\
                d\
            (* end *)",
        );
        let store = Store::new().with_must("left", 101).with_must("name", "");

        assert_eq!(engine.render(&template, &store).unwrap(), "c");
    }

    #[test]
    fn test_render_for() {
        let (template, engine) = get_template_with_engine(
            "(* for value in first *)\
                first loop: (( value )) \
                (* for value in second *)\
                    second loop: (( value )) \
                (* end *)\
            (* end *)",
        );
        let store = Store::new()
            .with_must("first", "ab")
            .with_must("second", "cd");

        assert_eq!(
            engine.render(&template, &store).unwrap(),
            "first loop: a second loop: c second loop: d \
            first loop: b second loop: c second loop: d "
        );
    }

    #[test]
    fn test_render_for_array() {
        let (template, engine) = get_template_with_engine(
            "(* for index, value in data *)\
                (( index )) - (( value )) \
            (* end *)",
        );
        let store = Store::new().with_must("data", json!(["one", "two"]));

        assert_eq!(
            engine.render(&template, &store).unwrap(),
            "0 - one 1 - two "
        );
    }

    #[test]
    fn test_render_for_object_pair() {
        let (template, engine) = get_template_with_engine(
            "(* for key, value in data *)\
                (( key )) - (( value ))\
            (* end *)",
        );
        let store = Store::new().with_must("data", json!({"one": "two"}));

        assert_eq!(engine.render(&template, &store).unwrap(), "one - two");
    }

    #[test]
    fn test_render_for_object_single() {
        let (template, engine) = get_template_with_engine(
            "(* for value in data *)\
                (( value ))\
            (* end *)",
        );
        let store = Store::new().with_must("data", json!({"one": "two"}));

        assert_eq!(engine.render(&template, &store).unwrap(), "two");
    }

    #[test]
    fn test_let_global_scope() {
        let (template, engine) = get_template_with_engine(
            "(* if is_admin *)\
                (* let name = \"admin\" *)\
            (* else *)\
                (* let name = user.name *)\
            (* end *)\
            Hello, (( name )).",
        );
        let store = Store::new()
            .with_must("is_admin", false)
            .with_must("user", json!({"name": "taylor"}));

        assert_eq!(engine.render(&template, &store).unwrap(), "Hello, taylor.");
    }

    #[test]
    fn test_let_scoped_dropped() {
        let (template, engine) = get_template_with_engine(
            "(* for item in inventory *)\
                (* let name = item.description.name *)\
                Item: (( name ))\
            (* end *)\
            Last item name: (( name )).",
        );
        let store =
            Store::new().with_must("inventory", json!([{"description": {"name": "sword"}}]));

        assert!(engine.render(&template, &store).is_err());
    }

    #[test]
    fn test_include() {
        let mut engine = Engine::default();
        engine
            .add_template_must("first", "two three (( name ))")
            .unwrap();
        engine
            .add_template_must("second", "one (* include first name: info.name *)")
            .unwrap();
        let store = Store::new().with_must("info", json!({"name": "taylor", "age": 28}));

        let template = engine.get_template("second").unwrap();

        assert_eq!(
            engine.render(&template, &store).unwrap(),
            "one two three taylor"
        );
    }

    #[test]
    fn test_extend() {
        let mut engine = Engine::default();
        engine
            .add_template_must("first", "hello, (* block name *)(* end *)!")
            .unwrap();
        engine
            .add_template_must(
                "second",
                "(* extends first *)\
                (* block name *)\
                (( name ))\
                (* end *)",
            )
            .unwrap();
        let store = Store::new().with_must("name", "taylor");
        let template = engine.get_template("second").unwrap();

        assert_eq!(engine.render(&template, &store).unwrap(), "hello, taylor!");
    }

    /// A helper function that returns a [`Template`] from the given text,
    /// and the [`Engine`] that compiled it.
    ///
    /// This function will unwrap the result of the compilation, so it should
    /// only be used on text that is expected to compile successfully.
    fn get_template_with_engine(text: &str) -> (Template, Engine) {
        let engine = Engine::default();
        (engine.compile(text).unwrap(), engine)
    }
}
