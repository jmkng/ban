mod compare;

use crate::{
    compile::{
        tree::{Arguments, Base, Call, CheckBranch, Expression, Key, Output, Tree},
        Scope, Template,
    },
    log::{error_write, Error, INVALID_FILTER},
    pipe::Pipe,
    region::Region,
    Engine, Store,
};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, fmt::Write};

use self::compare::{compare_values, is_truthy};

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
    Renderer::new(&Engine::default(), template, store).render()
}

pub struct Renderer<'source, 'store> {
    /// An engine containing any registered filters.
    engine: &'source Engine<'source>,
    /// The template being rendered.
    template: &'source Template<'source>,
    /// The Store that the Template is rendered with.
    store: &'store Store,
    /// Storage for Block tags.
    blocks: Vec<(String, Scope)>,
}

impl<'source, 'store> Renderer<'source, 'store> {
    /// Create a new Renderer.
    pub fn new(
        engine: &'source Engine,
        template: &'source Template<'source>,
        store: &'store Store,
    ) -> Self {
        Renderer {
            engine,
            template,
            store,
            blocks: vec![],
        }
    }

    /// Render the [`Template`] stored inside the [`Renderer`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if rendering any of the [`Tree`] instances within the `Template`
    /// fails, or writing the rendered `Tree` to the buffer fails.
    pub fn render(&mut self) -> Result<String, Error> {
        let mut buffer = String::with_capacity(self.template.source.len());
        let mut pipe = Pipe::new(&mut buffer);

        self.render_scope(&self.template.scope, &mut pipe)?;
        Ok(buffer)
    }

    /// Render the given [`Scope`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any of the [`Tree`] instances in the `Scope` cannot be rendered.
    fn render_scope(&self, scope: &Scope, pipe: &mut Pipe) -> Result<(), Error> {
        let mut tree = scope.data.iter();
        'render: while let Some(next) = tree.next() {
            match next {
                Tree::Raw(r) => {
                    let value = self.evaluate_raw(r)?;
                    pipe.write_str(value).map_err(|_| error_write())?
                },
                Tree::Output(o) => {
                    let value = self.evaluate_output(o)?;
                    pipe.write_value(&value).map_err(|_| error_write())?
                },
                Tree::If(i) => {
                    for branch in i.tree.branches.iter() {
                        if !self.evaluate_branch(branch)? {
                            if i.else_branch.is_some() {
                                self.render_scope(i.else_branch.as_ref().unwrap(), pipe)?;
                            }
                            continue 'render;
                        }
                    }
                    self.render_scope(&i.then_branch, pipe)?;
                },
                _ => todo!()
                // Tree::Include(_) => todo!(),
                // Tree::ForLoop(_) => todo!(),
            }
        }

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
                                self.template.source,
                                check.left.get_region().combine(base.get_region()),
                            )
                        })?
                    {
                        return Ok(false);
                    }
                }
                None => {
                    let result = if check.negate {
                        !is_truthy(&left)
                    } else {
                        is_truthy(&left)
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
    /// Returns an error if rendering the `Base` fails, which may happen when
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
    /// Returns an [`Error`] in these cases:
    ///
    /// - Rendering the `Base` of the `Call` chain fails.
    /// - Executing a [`Filter`] returns an `Error`.
    fn evaluate_call(&self, call: &'store Call) -> Result<Cow<Value>, Error> {
        let mut call_stack = vec![call];
        let mut begin: &Expression = &call.receiver;

        while let Expression::Call(call) = begin {
            call_stack.push(call);
            begin = &call.receiver;
        }
        let mut value = match begin {
            Expression::Base(base) => self.evaluate_base(base)?,
            _ => unreachable!(),
        };

        for call in call_stack.iter().rev() {
            let name_literal = call.name.region.literal(self.template.source)?;
            let func = self.engine.get_filter(name_literal);
            if func.is_none() {
                return Err(Error::build(INVALID_FILTER)
                    .pointer(self.template.source, call.name.region)
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
                .or_else(|e| Err(e.pointer(self.template.source, call.name.region)))?;

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
        Ok(region.literal(self.template.source)?)
    }

    /// Evaluate a set of [`Key`] instances to return a [`Value`] from the [`Store`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when accessing the literal value of the [`Region`]
    /// from any of the `Key` instances fails.
    fn evaluate_keys(&self, keys: &Vec<Key>) -> Result<Cow<Value>, Error> {
        let first_region = keys
            .first()
            .expect("key vector should always have at least one key")
            .get_region();
        let first_value = first_region.literal(self.template.source)?;
        let store_value = self.store.get(first_value);
        let mut value: Cow<Value> = if store_value.is_some() {
            Cow::Borrowed(store_value.unwrap())
        } else {
            // TODO Maybe error here instead of returning a null value,
            // but it should probably be configurable.
            Cow::Owned(Value::Null)
        };

        for key in keys.iter().skip(1) {
            match value.as_object() {
                Some(object) => {
                    let key_region = key.get_region();
                    let key_name = key_region.literal(self.template.source)?;
                    let next_object = object.get(key_name);

                    value = if next_object.is_some() {
                        Cow::Owned(next_object.unwrap().clone())
                    } else {
                        // TODO See TODO above ^
                        Cow::Owned(Value::Null)
                    };
                }
                None => return Ok(Cow::Owned(Value::Null)),
            }
        }

        Ok(value)
    }

    /// Evaluate an [`Arguments`] instance to return a [`HashMap`] that contains the same values.
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
            let name = if arg.0.is_some() {
                arg.0.unwrap().literal(self.template.source)?.to_string()
            } else {
                let temp = unnamed;
                unnamed += 1;
                temp.to_string()
            };

            let value = self.evaluate_base(&arg.1)?;
            buffer.insert(name, value.into_owned());
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::{compile::Parser, Engine, Store};

    #[test]
    fn test_render_raw() {
        let result = Renderer::new(
            &Engine::default(),
            &Parser::new("hello there").compile().unwrap(),
            &Store::new(),
        )
        .render();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello there");
    }

    #[test]
    fn test_render_output() {
        let result = Renderer::new(
            &Engine::default(),
            &Parser::new("hello there, (( name ))!").compile().unwrap(),
            &Store::new().with_must("name", "taylor"),
        )
        .render();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello there, taylor!");
    }

    #[test]
    fn test_render_if() {
        let result = Renderer::new(
            &Engine::default(),
            &Parser::new(
                "(* if left > 300 *)a\
                (* else if name == \"taylor\" *)b\
                (* else if not false *)c\
                (* else *)d\
                (* endif *)",
            )
            .compile()
            .unwrap(),
            &Store::new().with_must("left", 101).with_must("name", ""),
        )
        .render();
        assert_eq!(result.unwrap(), "c");
    }
}
