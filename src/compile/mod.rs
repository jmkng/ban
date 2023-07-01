use self::{lexer::Token, parser::Parser, template::Template};
use crate::types::{Region, Result};

mod lexer;
mod parser;
mod template;

pub type LexResult = Result<Option<Region<Token>>>;

pub fn compile<'source>(text: &'source str) -> Result<Template> {
    Parser::new(text).compile()
}
