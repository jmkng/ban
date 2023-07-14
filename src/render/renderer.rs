use crate::{
    compile::{
        tree::{Arguments, Base, Call, Expression, Key, Output, Tree},
        Scope, Template,
    },
    general_error, Context, Engine, Error, Formatter, Region,
};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, fmt::Write};

pub struct Renderer<'source, 'context> {
    /// An engine containing any registered filters.
    engine: &'source Engine<'source>,
    /// The template being rendered.
    template: Template<'source>,
    /// The context data that the template is checked against.
    context: &'context Context,
    /// Storage for Block tags.
    blocks: Vec<(String, Scope)>,
}

impl<'source, 'context> Renderer<'source, 'context> {
    /// Create a new Renderer.
    pub fn new(
        engine: &'source Engine,
        template: Template<'source>,
        context: &'context Context,
    ) -> Self {
        Renderer {
            engine,
            template,
            context,
            blocks: vec![],
        }
    }

    /// Render the Template stored inside the Renderer.
    ///
    /// # Errors
    ///
    /// TODO
    pub fn render(&mut self) -> Result<String, Error> {
        let mut buffer = String::with_capacity(self.template.source.len());
        let mut formatter = Formatter::new(&mut buffer);

        let mut tokens = self.template.scope.tokens.iter();
        while let Some(next) = tokens.next() {
            match next {
                Tree::Raw(raw) => {
                    let value = self.render_raw(raw)?;
                    formatter.write_str(value)
                },
                Tree::Output(output) => {
                    let value = self.render_output(output)?;
                    formatter.write_value(&value)
                },
                _ => todo!()
                // Tree::Include(_) => todo!(),
                // Tree::IfElse(_) => todo!(),
                // Tree::ForLoop(_) => todo!(),
            }
            .map_err(|_| Error::General("io error, failed to write to buffer".to_string()))?;
        }

        Ok(buffer)
    }

    /// Render an Output.
    ///
    /// # Errors
    ///
    /// TODO
    fn render_output(&self, output: &'source Output) -> Result<Cow<Value>, Error> {
        match &output.expression {
            Expression::Base(base) => self.render_base(base),
            Expression::Call(call) => self.render_call(call),
        }
    }

    /// Render a Base.
    ///
    /// TODO
    ///
    /// # Errors
    ///
    /// TODO
    fn render_base(&self, output: &'source Base) -> Result<Cow<Value>, Error> {
        match output {
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
    /// TODO
    fn render_call(&self, call: &'source Call) -> Result<Cow<Value>, Error> {
        let mut call_stack = vec![call];
        let mut begin: &Expression = &call.receiver;

        while let Expression::Call(call) = begin {
            call_stack.push(call);
            begin = &call.receiver;
        }
        let mut value = match begin {
            Expression::Base(base) => self.render_base(base),
            _ => unreachable!(),
        }?;

        for call in call_stack {
            let name_literal = call.name.region.literal(self.template.source)?;
            let func = self.engine.get_filter(name_literal);
            if func.is_none() {
                return general_error!(
                    "template has requested to use the filter `{}`, \
                    but filter is not registered in the engine, \
                    did you register the filter with [.add_filter | .add_filter_must]?",
                    name_literal
                );
            }

            let arguments = if call.arguments.is_some() {
                self.eval_arguments(call.arguments.as_ref().unwrap())?
            } else {
                HashMap::new()
            };

            value = Cow::Owned(
                func.unwrap()
                    .apply(&value, &arguments)
                    // TODO fix error
                    .map_err(|s| Error::General(s.to_string()))?,
            );
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
    /// TODO
    fn render_raw(&self, region: &Region) -> Result<&str, Error> {
        Ok(region.literal(self.template.source)?)
    }

    /// Get the Value from the Context which is identified by the given Keys.
    ///
    /// # Errors
    ///
    /// TODO
    fn eval_keys(&self, keys: &Vec<Key>) -> Result<Cow<Value>, Error> {
        let first_region = keys
            .first()
            .expect("key vector should always have at least one key")
            .get_region();
        let first_value = first_region.literal(self.template.source)?;

        // TODO Maybe error here instead. No config supported yet.
        let context_value = self.context.get(first_value);
        let mut value: Cow<Value> = if context_value.is_some() {
            Cow::Borrowed(context_value.unwrap())
        } else {
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
