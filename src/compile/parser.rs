//! Ash parser.
//!
//! Utilizes a lexer to receive instances of Region, which it uses to construct
//! a Template instance that contains the AST.
//!
//! This template can be combined with some context data to produce output.
//!
//! &str -> Vec<Region> -> Template -> String
//!         -----------------------
mod scope;
mod state;
mod token;

use crate::{
    compile::{
        lexer::{self, LexResult, Lexer},
        parser::scope::Scope,
        template::Template,
    },
    error::Error,
    region::Region,
};

use self::token::{Expression, LoopVariables};
use lexer::Token as LexerToken;
use token::Token as ParserToken;

type ParseResult = Result<(ParserToken, Region), Error>;

pub enum Block {
    If(bool, Expression),
    ElseIf(bool, Expression),
    Else,
    EndIf,
    For(LoopVariables, Expression),
    EndFor,
    EndWith,
    Include(String, Option<Expression>),
}

pub struct Parser<'source> {
    /// Lexer used to pull from source as tokens instead of raw text.
    source: Lexer<'source>,
    /// Store peeked tokens.
    ///
    /// Double option is used to remember when the next token is None.
    buffer: Option<Option<(LexerToken, Region)>>,
}

impl<'source> Parser<'source> {
    /// Create a new Parser from the given string.
    #[inline]
    pub fn new(source: &'source str) -> Self {
        Self {
            source: Lexer::new(source),
            buffer: None,
        }
    }

    /// Compile the given
    pub fn compile(mut self) -> Result<Template<'source>, Error> {
        // let mut states = vec![];
        let mut scopes = vec![Scope::new()];

        // while let Some((token, region)) = self.next()? {
        //     // Translate incoming Lexer::Token instances to Parser::Token.
        //     let region = match token {
        //         LexerToken::Raw => Ok((ParserToken::Raw, (region.begin..region.end).into())),
        //         LexerToken::BeginExpression => self.parse_expression(region.begin),
        //         LexerToken::BeginBlock => self.parse_block(region.begin),
        //         _ => panic!("unrecognized token surfaced in parser.compile"),
        //     };

        //     regions.push(region);
        // }

        todo!()
    }

    /// Parse a block.
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

    /// Parse an expression.
    fn parse_expression(&mut self, from: usize) -> ParseResult {
        // (( name | upper ))
        // |                 |
        // from              to
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
    use super::Parser;
    use crate::compile::lexer::Token;

    #[test]
    fn test_parser_lexer_integration() {
        let mut parser = Parser::new("hello");
        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }
}
