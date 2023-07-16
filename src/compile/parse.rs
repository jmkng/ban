//! Ash parser.
//!
//! Utilizes a Lexer to receive instances of Region, which it uses to construct
//! a new Template containing the Abstract Syntax Tree.
//!
//! This template can be combined with some Store data to produce output.
pub mod scope;
pub mod tree;

mod block;
mod state;

use crate::{
    compile::{
        lex::{token::Token, LexResult, LexResultMust, Lexer},
        parse::{
            block::Block,
            state::State,
            tree::{
                Arguments, Base, Call, Comparison, Expression, Identifier, Key, Literal, Output,
                Tree, Variable,
            },
        },
        Keyword, Operator,
    },
    log::{expected_keyword, INVALID_SYNTAX, UNEXPECTED_EOF, UNEXPECTED_TOKEN},
    Error, Pointer, Region, Scope, Template,
};
use serde_json::{Number, Value};
use std::ops::Range;

pub struct Parser<'source> {
    /// Lexer used to pull from source as tokens instead of raw text.
    lexer: Lexer<'source>,
    /// Store peeked tokens.
    ///
    /// Double option is used to remember when the next token is None.
    buffer: Option<Option<(Token, Region)>>,
}

impl<'source> Parser<'source> {
    /// Create a new Parser from the given string.
    #[inline]
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: Lexer::new(source),
            buffer: None,
        }
    }

    /// Compile the template.
    ///
    /// Returns a new Template, which can be executed with some Store
    /// data to receive output.
    pub fn compile(mut self) -> Result<Template<'source>, Error> {
        // Temporary storage for fragments of larger expressions.
        let mut states: Vec<State> = vec![];

        // Contains the distinct Tree instances within a specific area of the source.
        //
        // Used to remember what belongs to the if branch and what belongs to the else
        // branch in an "if" tag, for example.
        let mut scopes: Vec<Scope> = vec![Scope::new()];

        while let Some(next) = self.next()? {
            let tree = match next {
                (Token::Raw, region) => Tree::Raw(region),
                (Token::BeginExpression, region) => {
                    let expression = self.parse_expression()?;
                    let (_, next_region) = self.next_must(Token::EndExpression)?;
                    let merge = region.combine(next_region);
                    Tree::Output(Output::from((expression, merge)))
                }
                (Token::BeginBlock, region) => {
                    let block = self.parse_block()?;
                    todo!()
                }
                _ => todo!(),
            };

            scopes.last_mut().unwrap().data.push(tree);
        }

        if let Some(block) = states.first() {
            let (block, close, region) = match block {
                State::If { region, .. } => ("if", "endif", region),
                State::For { region, .. } => ("for", "endfor", region),
            };

            return Err(Error::build(INVALID_SYNTAX)
                .visual(Pointer::new(self.lexer.source, *region))
                .help(format!(
                    "did you close the `{block}` block with a `{close}` block?"
                )));
        }

        assert!(
            scopes.len() == 1,
            "parser should never have >1 scope after compilation"
        );

        Ok(Template {
            scope: scopes.remove(0),
            source: self.lexer.source,
        })
    }

    /// Parse a block.
    ///
    /// A block is a call to evaluate some kind of expression which may have
    /// side effects on the Store data.
    fn parse_block(&mut self) -> Result<Block, Error> {
        // from
        // |
        // (* if name == "taylor" *)
        //   Welcome back, Taylor.
        // (* endfor *)
        //             |
        //             to
        let (keyword, _) = self.parse_keyword()?;

        match keyword {
            Keyword::If => {
                let expression = self.parse_if()?;
                todo!();
                // Ok(Block::If(expression))
            }
            _ => todo!(),
        }
    }

    /// Parse an expression.
    ///
    /// An expression is a call to render some kind of data,
    /// and may contain one or more "filters" which are used to modify the output.
    fn parse_expression(&mut self) -> Result<Expression, Error> {
        // (( name | prepend 1: "hello, " | append "!" | upper ))
        // |                                                     |
        // from                                                  to
        let mut expression = Expression::Base(self.parse_base()?);

        while self.next_is(Token::Pipe)? {
            self.next_must(Token::Pipe)?;
            let name = self.parse_ident()?;
            let arguments = self.parse_args()?;

            let end_as: Region = if arguments.is_some() {
                arguments.as_ref().unwrap().region
            } else {
                name.region
            };
            let region = expression.get_region().combine(end_as);

            expression = Expression::Call(Call {
                name,
                arguments,
                receiver: Box::new(expression),
                region,
            })
        }

        Ok(expression)
    }

    /// Returns the parse negated base of this [`Parser`].
    ///
    /// Parse a Base, and include a boolean that is true when the token
    /// before the Base is an Exclamation.
    ///
    /// # Errors
    ///
    /// Returns an Error if the token before the Base is an Exclamation
    /// but the Base and Exclamation are not neighbors.
    ///
    /// ! name <- invalid
    ///
    /// !name <- valid
    fn parse_negated_base(&mut self) -> Result<(bool, Base), Error> {
        if self.next_is(Token::Exclamation)? {
            let exclamation = self.next_must(Token::Exclamation)?;
            let base = self.parse_base()?;
            if !base.get_region().is_neighbor(exclamation.1) {
                todo!() // return err
            }
            Ok((true, base))
        } else {
            Ok((false, self.parse_base()?))
        }
    }

    /// TODO
    fn parse_if(&mut self) -> Result<(bool, Comparison), Error> {
        // Input variants:
        //
        // this == that *)
        // !this *)
        let left = self.parse_negated_base()?;
        match self.next_any_must()? {
            (token, region) => match token {
                Token::Operator(operator) => {
                    let right = self.parse_negated_base()?;
                }
                _ => todo!(),
            },
        }
        let right = self.parse_negated_base()?;
        todo!()
    }

    /// Parse a Keyword.
    ///
    /// # Errors
    ///
    /// Returns an error if the next token is not a Keyword.
    fn parse_keyword(&mut self) -> Result<(Keyword, Region), Error> {
        match self.next_any_must()? {
            (Token::Keyword(keyword), region) => Ok((keyword, region)),
            (token, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .help(expected_keyword(token))
                .visual(Pointer::new(self.lexer.source, region))),
        }
    }

    /// Parse an Arguments.
    ///
    /// A filter's arguments may come in two different forms, named or anonymous.
    ///
    /// ## Named
    ///
    /// Named arguments have an explicit name. An argument name is an identifier
    /// followed by a colon (:), and is always treated as a string.
    ///
    /// In this example, the name of the argument is "1" and the value is "hello, ".
    ///
    /// 1: "hello, "
    ///
    /// ## Anonymous
    ///
    /// Anonymous arguments have no explicitly assigned name, however they do still have
    /// implicitly assigned names. View the filter module for more information on that.
    ///
    /// Here is an example of an anonymous argument:
    ///
    /// "hello, "
    fn parse_args(&mut self) -> Result<Option<Arguments>, Error> {
        // Input variants:
        //
        // 1: "hello, " |
        // "!" |
        // |
        // ))
        let mut values: Vec<(Option<Region>, Base)> = vec![];

        while !self.next_is(Token::Pipe)? && !self.next_is(Token::EndExpression)? {
            let name_or_value = self.parse_base()?;

            if self.next_is(Token::Colon)? {
                self.next_must(Token::Colon)?;

                let value = self.parse_base()?;
                values.push((Some(name_or_value.get_region()), value))
            } else {
                values.push((None, name_or_value))
            }
        }
        if values.is_empty() {
            return Ok(None);
        }

        // Gets the true Region of the given argument.
        let get_region = |pair: &(Option<Region>, Base)| {
            if pair.0.is_some() {
                pair.0.unwrap().combine(pair.1.get_region())
            } else {
                pair.1.get_region()
            }
        };

        let first = values.first().unwrap();
        let mut region = get_region(first);

        if values.len() > 1 {
            let last = values.last().unwrap();

            let last_region = get_region(last);
            region = region.combine(last_region)
        }

        Ok(Some(Arguments { values, region }))
    }

    /// Parse an Identifier.
    ///
    /// # Errors
    ///
    /// Propagates an error from next_must if the next token is not an
    /// Identifier.
    fn parse_ident(&mut self) -> Result<Identifier, Error> {
        let (_, region) = self.next_must(Token::Identifier)?;
        Ok(Identifier { region })
    }

    /// Parse a Base.
    ///
    /// A Base may be returned as a Literal or Variable based on the value.
    ///
    /// ## Literal
    ///
    /// "hello world"
    ///
    /// -1000
    ///
    /// 1000
    ///
    /// 10.2
    ///
    /// ## Variable
    ///
    /// person.name
    fn parse_base(&mut self) -> Result<Base, Error> {
        let expression = match self.next_any_must()? {
            (Token::Keyword(_), region) => {
                let literal = self.parse_bool_literal(region)?;
                Base::Literal(literal)
            }
            (Token::Operator(operator), region) => match operator {
                Operator::Add | Operator::Subtract => {
                    let (_, next_region) = self.next_must(Token::Number)?;

                    // -1000 | +1000  <- valid, negative/positive numbers
                    // - 1000 | + 1000<- invalid
                    if !region.is_neighbor(next_region) {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .visual(Pointer::new(self.lexer.source, region))
                            .help(format!(
                                "if you want to indicate that {} is a positive or negative number \
                                try removing the separating whitespace",
                                &self.lexer.source[next_region]
                            )));
                    }

                    let merge = region.combine(next_region);
                    let literal = self.parse_number_literal(&self.lexer.source[merge], merge)?;
                    Base::Literal(literal)
                }
                _ => {
                    return Err(Error::build(UNEXPECTED_TOKEN)
                        .visual(Pointer::new(self.lexer.source, region))
                        .help(format!(
                            "only `{}` or `{}` operators to indicate a positive or negative numbers \
                            are valid here",
                            Operator::Add,
                            Operator::Subtract
                        )));
                }
            },
            (Token::Number, region) => {
                let literal = self.parse_number_literal(&self.lexer.source[region], region)?;
                Base::Literal(literal)
            }
            (Token::String, region) => {
                let literal = self.parse_string_literal(region)?;
                Base::Literal(literal)
            }
            (Token::Identifier, region) => {
                let mut path = vec![Key::from(Identifier { region })];

                // Keep chaining keys as long as we see a period.
                while self.next_is(Token::Period)? {
                    self.next_must(Token::Period)?;
                    path.push(self.parse_key()?);
                }
                Base::Variable(Variable { path })
            }
            (_, region) => {
                return Err(
                    Error::build(UNEXPECTED_TOKEN).visual(Pointer::new(self.lexer.source, region))
                )
            }
        };

        Ok(expression)
    }

    /// Parse a Literal containing a Value::String from the literal value
    /// of the given Region.
    ///
    /// # Errors
    ///
    /// Propagates an error from [parse_string()] if an unrecognized escape character
    /// is found.
    fn parse_string_literal(&mut self, region: Region) -> Result<Literal, Error> {
        let value = Value::String(self.parse_string(region)?);
        Ok(Literal { value, region })
    }

    /// Parse a Key.
    ///
    /// # Errors
    ///
    /// Returns an error if the next token is not a valid Identifier such as "one.two".
    fn parse_key(&mut self) -> Result<Key, Error> {
        match self.next_any_must()? {
            (Token::Identifier, region) => Ok(Key::from(Identifier { region })),
            (_, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .visual(Pointer::new(self.lexer.source, region))
                .help("expected an unquoted identifier such as `one.two`")),
        }
    }

    /// Parse a String from the literal value of the given Region.
    ///
    /// # Errors
    ///
    /// Returns an error if an unrecognized escape character is found.
    fn parse_string(&self, region: Region) -> Result<String, Error> {
        let window = region
            .literal(self.lexer.source)
            .expect("window over source should never fail");

        let string = if window.contains('\\') {
            let mut iter = window.char_indices().map(|(i, c)| (region.begin + i, c));
            let mut string = String::new();

            while let Some((_, c)) = iter.next() {
                match c {
                    '"' => continue,
                    '\\' => {
                        let (_, esc) = iter.next().unwrap();
                        let c = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            _ => {
                                return Err(Error::build("unexpected escape character")
                                    .visual(Pointer::new(self.lexer.source, region)))
                            }
                        };
                        string.push(c);
                    }
                    c => string.push(c),
                }
            }
            string
        } else {
            window[1..window.len() - 1].to_owned()
        };

        Ok(string)
    }

    /// Parse a Literal containing a Value::Number from the given Region.
    ///
    /// # Errors
    ///
    /// Returns an error if the literal value of the Region cannot be converted
    /// to a Value::Number.
    fn parse_number_literal(&self, window: &str, region: Region) -> Result<Literal, Error> {
        let as_number: Number = window.parse().map_err(|_| {
            Error::build("unrecognizable number")
                .visual(Pointer::new(self.lexer.source, region))
                .help(format!(
                    "numbers may begin with `{}` to indicate a negative \
                    number and must not end with a decimal",
                    Operator::Subtract
                ))
        })?;

        Ok(Literal {
            value: Value::Number(as_number),
            region,
        })
    }

    /// Return a Literal containing a Value::Bool from the Region.
    ///
    /// # Errors
    ///
    /// If the Region does not point to a literal "true" or "false", an error is returned.
    fn parse_bool_literal(&mut self, region: Region) -> Result<Literal, Error> {
        let value: &str = self
            .lexer
            .source
            .get::<Range<usize>>(region.into())
            .expect("window over source should always exist");

        let bool = match value {
            "true" => true,
            "false" => false,
            _ => {
                return Err(Error::build("unrecognizable boolean")
                    .visual(Pointer::new(self.lexer.source, region))
                    .help("expected `true` or `false`"))
            }
        };

        Ok(Literal {
            value: Value::Bool(bool),
            region,
        })
    }

    /// Peek the next token.
    ///
    /// # Errors
    ///
    /// Propagates any error reported by the underlying Lexer.
    fn peek(&mut self) -> LexResult {
        if let o @ None = &mut self.buffer {
            *o = Some(self.lexer.next()?);
        }

        Ok(self.buffer.unwrap())
    }

    /// Get the next token.
    ///
    /// Prefers to pull a token from the internal buffer first, but will pull from
    /// the lexer when the buffer is empty.
    fn next(&mut self) -> LexResult {
        match self.buffer.take() {
            Some(t) => Ok(t),
            None => self.lexer.next(),
        }
    }

    /// Returns true if the given token matches the upcoming token.
    ///
    /// # Errors
    ///
    /// Propagates any errors reported by the underlying lexer.
    fn next_is(&mut self, expect: Token) -> Result<bool, Error> {
        Ok(self
            .peek()?
            .map(|(token, _)| token == expect)
            .unwrap_or(false))
    }

    /// Get the next token, and compare it to the given token.
    ///
    /// # Errors
    ///
    /// An error is returned if the next token does not match the given token,
    /// or when [next()] returns None.
    fn next_must(&mut self, expect: Token) -> LexResultMust {
        match self.next()? {
            Some((token, region)) => {
                if token == expect {
                    Ok((token, region))
                } else {
                    Err(Error::build(UNEXPECTED_TOKEN)
                        .visual(Pointer::new(self.lexer.source, region))
                        .help(format!("expected `{expect}`")))
                }
            }
            None => {
                let source_len = self.lexer.source.len();
                Err(Error::build(UNEXPECTED_EOF)
                    .visual(Pointer::new(
                        self.lexer.source,
                        (source_len..source_len).into(),
                    ))
                    .help(format!("expected `{expect}`")))
            }
        }
    }

    /// Get the next token.
    ///
    /// Similar to "next()" but requires that a token is returned.
    ///
    /// # Errors
    ///
    /// An error is returned if no more tokens are left.
    fn next_any_must(&mut self) -> LexResultMust {
        match self.next()? {
            Some((token, region)) => Ok((token, region)),
            None => {
                let source_len = self.lexer.source.len();
                return Err(Error::build(UNEXPECTED_EOF)
                    .visual(Pointer::new(
                        self.lexer.source,
                        (source_len..source_len).into(),
                    ))
                    .help("expected additional tokens, did you make sure all blocks and expressions are closed?"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::compile::lex::token::Token;

    #[test]
    fn test_parser_lexer_integration() {
        let mut parser = Parser::new("hello");
        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }

    #[test]
    fn test_parse_full_expression() {
        let text = "hello (( name | prepend 1: \"hello, \" | append \"!\" | upper ))";
        let result = Parser::new(text).compile();
        assert!(result.is_ok());
        // println!("{:#?}", result.unwrap().scope.tokens);
        // println!("{}", text.get(6..60).unwrap())
    }

    #[test]
    fn test_parse_negative_num_err() {
        let text = "balance: (( - 1000 ))";
        let result = Parser::new(text).compile();
        assert!(result.is_err(),);
    }

    #[test]
    fn test_peek_multiple() {
        let text = "(( one two";
        let mut parser = Parser::new(text);
        assert!(parser.next().is_ok());
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
    }
}
