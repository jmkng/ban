pub mod filter;
pub mod pipe;

mod compare;
mod store;

pub use store::Store;

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Write},
    mem::take,
};

use crate::{
    compile::{tree::*, Scope, Template},
    engine::INVALID_FILTER,
    log::Error,
    region::Region,
    Engine,
};

use self::{
    compare::{compare_values, is_truthy},
    pipe::Pipe,
    store::Shadow,
};

use serde::Serialize;
use serde_json::Value;

const INCOMPATIBLE_TYPES: &str = "incompatible types";

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
    /// Returns an [`Error`] if rendering any [`Tree`] instance fails,
    /// or writing to the [`Pipe`] fails.
    pub fn render(mut self, pipe: &mut Pipe) -> Result<(), Error> {
        match &self.template.get_extends() {
            Some(extended) => self.evaluate_scope(extended, pipe),
            None => self.render_scope(self.template.get_scope(), pipe),
        }
        .map_err(|error| {
            // The `Error` might come from another `Template`, so don't change
            // the name if it already has one.
            if !error.get_name().is_some() && self.template.get_name().is_some() {
                return error.with_name(self.template.get_name().unwrap());
            }

            error
        })?;

        Ok(())
    }

    /// Evaluate the [`Scope`] and collect all [`Block`] instances within.
    ///
    /// After all `Block` instances are collected, a new [`Renderer`] is created and used
    /// to render the extended [`Template`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the extended `Template` does not exist, or rendering any
    /// [`Tree`] instance fails.
    fn evaluate_scope(&mut self, extends: &Extends, pipe: &mut Pipe) -> Result<(), Error> {
        let name = extends
            .name
            .get_region()
            .literal(self.template.get_source());
        let template = self
            .engine
            .get_template(name)
            .ok_or_else(|| error_missing_template(name))?;
        self.collect_blocks(self.template.get_scope());

        Renderer::new(self.engine, &template, self.shadow.store)
            .with_blocks(take(&mut self.blocks))
            .render(pipe)
    }

    /// Render a [`Scope`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering any [`Tree`] instance fails.
    fn render_scope(&mut self, scope: &'source Scope, pipe: &mut Pipe) -> Result<(), Error> {
        let mut iterator = scope.data.iter();
        while let Some(next) = iterator.next() {
            match next {
                Tree::Raw(ra) => {
                    let value = self.evaluate_raw(ra);
                    pipe.write_str(value).map_err(|_| error_write())?
                }
                Tree::Output(ou) => {
                    let value = self.evaluate_expression(&ou.expression)?;
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
                _ => unreachable!("parser must catch invalid top level tree"),
            }
        }

        Ok(())
    }

    /// Render a [`Block`] within the [`Renderer`] that matches the name of the given `Block`.
    ///
    /// When no matching `Block` exists, renders the scope within the `Block` itself.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering any [`Tree`] instance fails.
    fn render_block(&mut self, block: &'source Block, pipe: &mut Pipe) -> Result<(), Error> {
        let name = block.name.get_region().literal(self.template.get_source());

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
    /// or rendering any [`Tree`] instance fails.
    fn render_include(&mut self, include: &Include, pipe: &mut Pipe) -> Result<(), Error> {
        let name = include
            .name
            .get_region()
            .literal(self.template.get_source());
        let template = self
            .engine
            .get_template(name)
            .ok_or_else(|| error_missing_template(name))?;

        if include.mount.is_some() {
            // Scoped include, create a new store that includes only the named values.
            let mut scoped_store = Store::new();

            for point in include.mount.as_ref().unwrap().values.iter() {
                let name = point.name.literal(self.template.get_source());
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

    /// Render a [`For`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the [`Base`] is not found in the [`Store`],
    /// or rendering any [`Tree`] instance fails.
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
                return Err(Error::build(INCOMPATIBLE_TYPES).with_help(format!(
                    "iterating on value `{}` is not supported",
                    incompatible
                )))
            }
        }
        self.shadow.pop();

        Ok(())
    }

    /// Return true if the entire [`IfBranch`] is truthy.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a [`Value`] that the `IfBranch` depends on does
    /// not exist in the [`Store`].
    fn evaluate_branch(&self, branch: &IfBranch) -> Result<bool, Error> {
        for leaf in branch {
            let left = self.evaluate_base(&leaf.left)?;

            match &leaf.right {
                Some(base) => {
                    let right = self.evaluate_base(&base)?;
                    let operator = leaf
                        .operator
                        .expect("operator must exist when leaf.right exists");

                    if !compare_values(&left, operator, &right)
                        .map(|bool| if leaf.negate { !bool } else { bool })
                        .map_err(|error| {
                            error.with_pointer(
                                self.template.get_source(),
                                leaf.left.get_region().combine(base.get_region()),
                            )
                        })?
                    {
                        return Ok(false);
                    }
                }
                None => {
                    let result = match leaf.negate {
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
    /// Returns an [`Error`] if rendering the `Output` fails.
    fn evaluate_expression(&self, expression: &'source Expression) -> Result<Cow<Value>, Error> {
        match &expression {
            Expression::Base(base) => self.evaluate_base(base),
            Expression::Call(call) => self.evaluate_call(call),
        }
    }

    /// Evaluate a [`Base`] to return a [`Value`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a [`Value`] that the `Base` depends on does not exist
    /// in the [`Store`].
    fn evaluate_base(&self, base: &'source Base) -> Result<Cow<Value>, Error> {
        match base {
            Base::Variable(variable) => self.evaluate_keys(&variable.path),
            Base::Literal(literal) => Ok(Cow::Borrowed(&literal.value)),
        }
    }

    /// Evaluate a [`Call`] to return a [`Value`].
    ///
    /// Determines the initial input to the first [`Filter`][`crate::filter::Filter`]
    /// by following the receiver until a [`Base`] is found, and deriving the input
    /// from that `Base`.
    ///
    /// The output of the final `Call` in the chain is the return value.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when rendering the `Base` of the `Call` chain fails,
    /// or a `Filter` returns an [`Error`].
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
            let name_literal = call.name.region.literal(self.template.get_source());
            let func = self.engine.get_filter(name_literal);
            if func.is_none() {
                return Err(Error::build(INVALID_FILTER)
                    .with_pointer(self.template.get_source(), call.name.region)
                    .with_help(format!(
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

            let returned = func.unwrap().apply(&value, &arguments).or_else(|error| {
                Err(error.with_pointer(self.template.get_source(), call.name.region))
            })?;

            value = Cow::Owned(returned);
        }

        Ok(value)
    }

    /// Evaluate a [`Region`] to return a `&str`.
    ///
    /// The literal value of the `Region` within the source text is retrieved
    /// and returned.
    fn evaluate_raw(&self, region: &Region) -> &str {
        region.literal(self.template.get_source())
    }

    /// Evaluate a set of [`Identifier`] instances to return a [`Value`] from the [`Store`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a `Value` that an `Identifier` depends on does not exist in
    /// the `Store`.
    fn evaluate_keys(&self, keys: &Vec<Identifier>) -> Result<Cow<Value>, Error> {
        let first_region = keys
            .first()
            .expect("key vector should always have at least one key")
            .region;

        let first_value = first_region.literal(self.template.get_source());
        let store_value = self.shadow.get(first_value);

        let mut value: Cow<Value> = if store_value.is_some() {
            Cow::Borrowed(store_value.unwrap())
        } else {
            return Err(Error::build("missing store value")
                .with_pointer(self.template.get_source(), first_region)
                .with_help(format!(
                    "unable to find `{first_value}` in store, \
                    ensure it exists or try wrapping with an `if` block",
                )));
        };

        for key in keys.iter().skip(1) {
            match value.as_object() {
                Some(object) => {
                    let key_region = key.region;
                    let key_name = key_region.literal(self.template.get_source());
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
    /// As described in the [`filter`][`crate::filter`] module, any argument without a name
    /// will be automatically assigned a name.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering a [`Base`] fails.
    fn evaluate_arguments(&self, arguments: &Arguments) -> Result<HashMap<String, Value>, Error> {
        let mut buffer = HashMap::new();
        let mut unnamed = 1;

        for arg in &arguments.values {
            let name = if arg.name.is_some() {
                arg.name
                    .unwrap()
                    .literal(self.template.get_source())
                    .to_string()
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
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a [`Value`] that the [`Base`] depends on does not
    /// exist in the [`Store`].
    fn evaluate_let(&mut self, le: &Let) -> Result<(), Error> {
        let value = self.evaluate_expression(&le.right)?;
        self.shadow_set(
            &Set::Single(le.left.clone()),
            (None::<Value>, value.into_owned()),
        )?;

        Ok(())
    }

    /// Set the blocks property on the [`Renderer`].
    ///
    /// Returns the `Renderer`, so additional methods may be chained.
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
                    let name = block.name.get_region().literal(self.template.get_source());
                    self.blocks.insert(
                        name.to_string(),
                        Named {
                            template: self.template,
                            block: block.clone(),
                        },
                    );
                }
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
        let source = self.template.get_source();
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

/// Return an [`Error`] describing a missing template.
fn error_missing_template(name: &str) -> Error {
    Error::build("missing template").with_help(format!(
        "template `{}` not found in engine, add it with `.add_template`",
        name
    ))
}

/// Return an [`Error`] explaining that the write operation failed.
///
/// This is likely caused by a failure during a `write!` macro operation.
fn error_write() -> Error {
    Error::build("write failure")
        .with_help("failed to write result of render, are you low on memory?")
}

type BlockMap<'source> = HashMap<String, Named<'source>>;

/// A wrapper for [`Block`] that includes a reference to the [`Template`]
/// that the `Block` was found in.
struct Named<'source> {
    /// The [`Template`] that the [`Block`] was found in.
    template: &'source Template,
    /// A [`Block`] found in a [`Template`].
    block: Block,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        compile::tree::{Argument, Arguments, Base, Literal},
        filter::Error,
        Engine, Store, Template,
    };

    use super::Renderer;

    use serde_json::{json, Value};

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
    fn test_render_nested_for() {
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
        let engine = Engine::default();
        let single = engine
            .compile(
                "(* for value in data *)\
                    (( value )) \
                (* end *)",
            )
            .unwrap();
        let pair = engine
            .compile(
                "(* for index, value in data *)\
                    (( index )) - (( value )) \
                (* end *)",
            )
            .unwrap();
        let store = Store::new().with_must("data", json!(["one", "two"]));

        assert_eq!(engine.render(&single, &store).unwrap(), "one two ");
        assert_eq!(engine.render(&pair, &store).unwrap(), "0 - one 1 - two ");
    }

    #[test]
    fn test_render_for_object_set() {
        let engine = Engine::default();
        let single = engine
            .compile(
                "(* for value in data *)\
                (( value ))\
            (* end *)",
            )
            .unwrap();
        let pair = engine
            .compile(
                "(* for key, value in data *)\
                (( key )) - (( value ))\
            (* end *)",
            )
            .unwrap();
        let store = Store::new().with_must("data", json!({"one": "two"}));

        assert_eq!(engine.render(&single, &store).unwrap(), "two");
        assert_eq!(engine.render(&pair, &store).unwrap(), "one - two");
    }

    #[test]
    fn test_let_global_scope_if() {
        let (template, mut engine) = get_template_with_engine(
            "(* if is_admin *)\
                (* let name = \"admin\" *)\
            (* else *)\
                (* let name = user.name | to_lowercase *)\
            (* end *)\
            Hello, (( name )).",
        );
        engine.add_filter_must("to_lowercase", to_lowercase);
        let store = Store::new()
            .with_must("is_admin", false)
            .with_must("user", json!({"name": "Taylor"}));

        assert_eq!(engine.render(&template, &store).unwrap(), "Hello, taylor.");
    }

    #[test]
    fn test_let_pop_scoped() {
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
    fn test_collect_blocks() {
        let (template, engine) =
            get_template_with_engine("(* block one *)one(* end *)(* block two *)two(* end *)");
        let store = Store::new();

        let mut renderer = Renderer::new(&engine, &template, &store);

        assert!(renderer.blocks.get("one").is_none());
        assert!(renderer.blocks.get("two").is_none());
        renderer.collect_blocks(template.get_scope());

        assert!(renderer.blocks.get("one").is_some());
        assert!(renderer.blocks.get("two").is_some());
        assert!(renderer.blocks.get("three").is_none());
    }

    #[test]
    fn test_evaluate_arguments_increment() {
        let (template, engine) = get_template_with_engine("");
        let store = Store::new();
        let renderer = Renderer::new(&engine, &template, &store);

        let arguments = renderer
            .evaluate_arguments(&Arguments {
                values: vec![
                    Argument {
                        name: None,
                        value: Base::Literal(Literal {
                            value: json!("hello"),
                            region: (0..0).into(),
                        }),
                    },
                    Argument {
                        name: None,
                        value: Base::Literal(Literal {
                            value: json!("goodbye"),
                            region: (0..0).into(),
                        }),
                    },
                ],
                region: (0..0).into(),
            })
            .unwrap();

        assert_eq!(arguments.get("1"), Some(&json!("hello")));
        assert_eq!(arguments.get("2"), Some(&json!("goodbye")));
    }

    /// An example filter used for testing.
    fn to_lowercase(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
        match value {
            Value::String(string) => Ok(json!(string.to_owned().to_lowercase())),
            _ => Err(Error::build("filter `to_lowercase` requires string input")
                .with_help("use quotes to coerce data to string")),
        }
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
