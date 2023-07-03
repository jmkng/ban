use self::{parser::Parser, template::Template};
use crate::types::Error;

mod lexer;
mod parser;
mod template;

pub fn compile<'source>(text: &'source str) -> Result<Template, Error> {
    Parser::new(text).compile()
}
