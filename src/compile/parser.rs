use crate::types::Result;

use super::{lexer::Lexer, template::Template};

pub struct Parser<'source> {
    source: Lexer<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            source: Lexer::new(source),
        }
    }

    pub fn compile(self) -> Result<Template<'source>> {
        todo!()
    }
}
