use crate::{
    compile::{
        tree::{Arguments, Base, Call, Expression, Key, Output, Tree},
        Scope, Template,
    },
    log::INVALID_FILTER,
    Engine, Error, Pipe, Pointer, Region, Store,
};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, fmt::Write};

pub struct Renderer<'source, 'store> {
    /// An engine containing any registered filters.
    engine: &'source Engine<'source>,
    /// The template being rendered.
    template: Template<'source>,
    /// The Store that the Template is rendered with.
    store: &'store Store,
    /// Storage for Block tags.
    blocks: Vec<(String, Scope)>,
}

impl<'source, 'store> Renderer<'source, 'store> {
    /// Create a new Renderer.
    pub fn new(engine: &'source Engine, template: Template<'source>, store: &'store Store) -> Self {
        Renderer {
            engine,
            template,
            store,
            blocks: vec![],
        }
    }

    /// Render the Template stored inside the Renderer.
    ///
    /// # Errors
    ///
    /// Returns an Error if rendering any of the Tree instances within the Template
    /// fails, or writing the rendered Tree to the buffer fails.
    pub fn render(&mut self) -> Result<String, Error> {
        let mut buffer = String::with_capacity(self.template.source.len());
        let mut pipe = Pipe::new(&mut buffer);

        let mut tokens = self.template.scope.data.iter();
        while let Some(next) = tokens.next() {
            match next {
                Tree::Raw(raw) => {
                    let value = self.render_raw(raw)?;
                    pipe.write_str(value)
                },
                Tree::Output(output) => {
                    let value = self.render_output(output)?;
                    pipe.write_value(&value)
                },
                _ => todo!()
                // Tree::Include(_) => todo!(),
                // Tree::IfElse(_) => todo!(),
                // Tree::ForLoop(_) => todo!(),
            }
            .map_err(|_| {
                Error::build("write failure")
                    .help("unable to continue rendering, system may be low on memory")
            })?;
        }

        Ok(buffer)
    }

    /// Render an Output.
    ///
    /// # Errors
    ///
    /// Returns an Error if rendering the Output fails.
    fn render_output(&self, output: &'source Output) -> Result<Cow<Value>, Error> {
        match &output.expression {
            Expression::Base(base) => self.render_base(base),
            Expression::Call(call) => self.render_call(call),
        }
    }

    /// Render a Base.
    ///
    /// # Errors
    ///
    /// Returns an Error if rendering the Base fails.
    fn render_base(&self, base: &'source Base) -> Result<Cow<Value>, Error> {
        match base {
            Base::Variable(variable) => self.eval_keys(&variable.path),
            Base::Literal(literal) => Ok(Cow::Borrowed(&literal.value)),
        }
    }

    /// Render a Call.
    ///
    /// Follows the receiver until a Base is reached, the beginning input
    /// is derived from this base.
    ///
    /// From there, we work in the opposite direction, calling each filter
    /// function one by one until we get back to the end of the Call.
    ///
    /// The output of the final Call in the chain is the return value.
    ///
    /// # Errors
    ///
    /// Returns an Error in these cases:
    ///
    /// - Rendering the Base of the Call chain fails.
    /// - Executing a Filter returns an Error.
    /// -
    fn render_call(&self, call: &'store Call) -> Result<Cow<Value>, Error> {
        let mut call_stack = vec![call];
        let mut begin: &Expression = &call.receiver;

        while let Expression::Call(call) = begin {
            call_stack.push(call);
            begin = &call.receiver;
        }

        let mut value = match begin {
            Expression::Base(base) => self.render_base(base)?,
            _ => unreachable!(),
        };

        for call in call_stack.iter().rev() {
            let name_literal = call.name.region.literal(self.template.source)?;
            let func = self.engine.get_filter(name_literal);
            if func.is_none() {
                return Err(Error::build(INVALID_FILTER)
                    .visual(Pointer::new(self.template.source, call.name.region))
                    .help(format!(
                        "template wants to use the `{name_literal}` filter, but a filter with that \
                        name was not found in this engine, did you add the filter to the engine with \
                        `.add_filter` or `.add_filter_must`?"
                    )));
            }

            let arguments = if call.arguments.is_some() {
                self.eval_arguments(call.arguments.as_ref().unwrap())?
            } else {
                HashMap::new()
            };

            let returned = func
                .unwrap()
                .apply(&value, &arguments)
                .or_else(|e| Err(e.visual(Pointer::new(self.template.source, call.name.region))))?;

            value = Cow::Owned(returned);
        }

        Ok(value)
    }

    /// Render a Region.
    ///
    /// The literal value of the Region within the source text is retrieved
    /// and returned.
    ///
    /// # Errors
    ///
    /// Returns an Error if accessing the literal value of the Region fails.
    fn render_raw(&self, region: &Region) -> Result<&str, Error> {
        Ok(region.literal(self.template.source)?)
    }

    /// Return the Value from the Store that exists at the path identified
    /// by the given keys.
    ///
    /// # Errors
    ///
    /// Returns an Error when accessing the literal value of the Region
    /// from any of the Key instances fails.
    fn eval_keys(&self, keys: &Vec<Key>) -> Result<Cow<Value>, Error> {
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

    /// Assemble a HashMap that contains the values of the given Arguments.
    ///
    /// As described in the filter module, any argument without a name will
    /// be automatically assigned a name.
    fn eval_arguments(&self, arguments: &Arguments) -> Result<HashMap<String, Value>, Error> {
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

            let value = self.render_base(&arg.1)?;
            buffer.insert(name, value.into_owned());
        }

        Ok(buffer)
    }
}
