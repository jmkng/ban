//! Ash parser.
//!
//! Utilizes a lexer to receive instances of Region, which it uses to construct
//! a Template instance that contains the AST.
//!
//! This template can be combined with some context data to produce output.
//!
//! &str -> Vec<Region> -> Template -> String
//!         -----------------------
mod token;

use super::lexer::{self, LexResult};
use super::{lexer::Lexer, template::Template};
use crate::types::{Error, Region};

use lexer::Token as LexerToken;
use token::Token as ParserToken;

type ParseResult = Result<(ParserToken, Region), Error>;

pub struct Parser<'source> {
    /// Lexer used to pull from source as tokens instead of raw text.
    source: Lexer<'source>,
    /// Store peeked tokens.
    ///
    /// Double option is used to remember when the next token is None.
    buffer: Option<Option<(LexerToken, Region)>>,
}

impl<'source> Parser<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            source: Lexer::new(source),
            buffer: None,
        }
    }

    pub fn compile(mut self) -> Result<Template<'source>, Error> {
        let mut regions = vec![];

        while let Some((token, region)) = self.next()? {
            // Translate incoming Lexer::Token instances to Parser::Token.
            let region: ParseResult = match token {
                lexer::Token::Raw => Ok((ParserToken::Raw, (region.begin..region.end).into())),
                lexer::Token::BeginExpression => self.parse_expression(region.begin),
                lexer::Token::BeginBlock => self.parse_block(region.begin),
                _ => panic!("unrecognized token surfaced in parser.compile"),
            };

            regions.push(region);
        }

        todo!()
    }

    fn parse_block(&mut self, from: usize) -> ParseResult {
        // from
        // |
        // (* if name == "taylor" *)
        //   Welcome back, Taylor.
        // (* endfor *)
        //             |
        //             to
        todo!();
    }

    fn parse_expression(&mut self, from: usize) -> ParseResult {
        // (( name ))
        // |         |
        // from      to
        todo!();
    }

    /// Peek the next token.
    fn peek(&mut self) -> LexResult {
        // If self.buffer is None, we must initialize it as Some.
        if let o @ None = &mut self.buffer {
            *o = Some(self.source.next()?);
        }
        Ok(self.buffer.unwrap())
    }

    /// Get the next token.
    ///
    /// Pulls from internal buffer first, and if that is empty
    /// pulls from the lexer.
    fn next(&mut self) -> LexResult {
        match self.buffer.take() {
            Some(t) => Ok(t),
            None => self.source.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compile::lexer::Token;

    use super::Parser;

    #[test]
    fn test_parser_lexer_integration() {
        let mut parser = Parser::new("hello");
        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }
}
