use self::{parser::Parser, template::Template};
use crate::types::Result;

mod lexer;
mod parser;
mod template;
mod token;
pub use token::Token;

pub fn compile<'source>(text: &'source str) -> Result<Template> {
    Parser::new(text).compile()
}
