pub mod scope;
pub mod tree;

mod block;
mod state;

use crate::{
    compile::{
        lex::{token::Token, LexResult, LexResultMust, Lexer},
        parse::{block::Block, state::State, tree::*},
        Keyword, Operator, Scope, Template,
    },
    log::{
        expected_keyword, expected_operator, unexpected_eof, Error, Pointer, INVALID_SYNTAX,
        UNEXPECTED_EOF, UNEXPECTED_TOKEN,
    },
    region::Region,
};
use serde_json::{Number, Value};
use std::ops::Range;

use self::state::CompareState;

pub struct Parser<'source> {
    /// `Lexer` used to pull from source as tokens instead of raw text.
    lexer: Lexer<'source>,
    /// Store peeked tokens.
    ///
    /// Double option is used to remember when the next token is None.
    buffer: Option<Option<(Token, Region)>>,
}

impl<'source> Parser<'source> {
    /// Create a new `Parser` from the given string.
    #[inline]
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: Lexer::new(source),
            buffer: None,
        }
    }

    /// Compile a [`Template`].
    ///
    /// Returns a new `Template`, which can be executed with some [`Store`][`crate::Store`]
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

    /// Parse a [`Block`].
    ///
    /// A `Block` is a call to evaluate some kind of expression which may have
    /// side effects on the Store data.
    fn parse_block(&mut self) -> Result<Block, Error> {
        // from
        // |
        // (* if name == "taylor" *)
        //   Welcome back, Taylor.
        // (* endfor *)
        //          |
        //          to
        let (keyword, _) = self.parse_keyword()?;

        match keyword {
            Keyword::If => {
                let compare = self.parse_compare()?;
                // TODO: Resume by expecting EndBlock
                todo!();
                // Ok(Block::If(compare))
            }
            _ => todo!(),
        }
    }

    /// Parse an [`Expression`].
    ///
    /// An `Expression` is a call to render some kind of data,
    /// and may contain one or more "filters" which are used to modify the output.
    fn parse_expression(&mut self) -> Result<Expression, Error> {
        // (( name | prepend 1: "hello, " | append "!" | upper ))
        // |                                                  |
        // from                                               to
        let mut expression = Expression::Base(self.parse_base()?);
        // TODO: Negation checks

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
    /// Parse a [`Base`], and include a boolean that is true when the token
    /// before the Base is an exclamation.
    ///
    /// # Errors
    ///
    /// Returns an error if the token before the `Base` is an exclamation
    /// but the `Base` and exclamation are not neighbors.
    ///
    /// ```text
    /// ! name <- invalid
    /// !name <- valid
    /// ```
    ///
    /// May also propagate an error that is generated while parsing the
    /// actual `Base`.
    fn parse_negated_base(&mut self) -> Result<Base, Error> {
        if self.next_is(Token::Exclamation)? {
            let exclamation = self.next_must(Token::Exclamation)?;
            let mut base = self.parse_base()?;

            if !base.get_region().is_neighbor(exclamation.1) {
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .visual(Pointer::new(
                        self.lexer.source,
                        exclamation.1.combine(base.get_region()),
                    ))
                    .help(format!(
                        "you can use `{}` to negate the base expression, \
                        but you must remove the separating whitespace",
                        Token::Exclamation
                    )));
            }
            base.set_negate(true);
            Ok(base)
        } else {
            let mut base = self.parse_base()?;
            base.set_negate(false);
            Ok(base)
        }
    }

    /// Parse a [`Compare`].
    ///
    /// This `Compare` will contain all of the information necessary
    /// to determine if the `if` block is considered true or false.
    ///
    /// Negated [`Base`] expressions are supported.
    fn parse_compare(&mut self) -> Result<Compare, Error> {
        // this >= that && these == those || a == b *)
        // |                                       |
        // from                                    to
        let mut compare = Compare::new();
        let mut state = CompareState::Base(false);

        loop {
            match state {
                // Parse a `Base` and assign it to either "left" or "right"
                // on the latest `Check` instance.
                CompareState::Base(has_left) => {
                    let base = self.parse_negated_base()?;
                    if has_left {
                        compare
                            .last_check_mut_must("has_left implies that a check exists")
                            .right = Some(base);
                        state = CompareState::Transition;
                    } else {
                        compare.last_mut_must().push(Check::new(base));
                        state = CompareState::Operator;
                    };
                }
                // Parse an `Operator` for the `Check`.
                //
                // Set state to CompareState::Transition if something like "&&", "||"
                // or Token::EndBlock shows up.
                CompareState::Operator => match self.peek_must()? {
                    (token, region) => match token {
                        Token::EndBlock => state = CompareState::Transition,
                        Token::Operator(operator) => match operator {
                            Operator::Assign => {
                                // This might be a common mistake, so better to provide a more
                                // detailed error.
                                return Err(Error::build(UNEXPECTED_TOKEN)
                                    .visual(Pointer::new(self.lexer.source, region))
                                    .help(format!(
                                    "`{}` is only valid in `let` block, did you mean to use `{}` \
                                    to check for equality?",
                                    Operator::Assign,
                                    Operator::Equal
                                )));
                            }
                            Operator::Or | Operator::And => state = CompareState::Transition,
                            operator => {
                                // Valid operators are handled here.
                                self.next_any_must()?;
                                compare
                                    .last_check_mut_must("operator state implies that check exists")
                                    .operator = Some(operator);

                                state = CompareState::Base(true);
                            }
                        },
                        token => {
                            return Err(Error::build(UNEXPECTED_TOKEN)
                                .visual(Pointer::new(self.lexer.source, region))
                                .help(expected_operator(token)))
                        }
                    },
                },
                // Handle "&&", "||" and `Token::EndBlock`.
                CompareState::Transition => match self.next_any_must()? {
                    (Token::EndBlock, _) => break,
                    (Token::Operator(operator), _) if operator == Operator::Or => {
                        // Split up the `Path`.
                        compare.split_path();
                        state = CompareState::Base(false);
                    }
                    (Token::Operator(operator), _) if operator == Operator::And => {
                        // Start a new `Check`, which requires a `Base`.
                        let base = self.parse_negated_base()?;
                        compare.split_check(base);
                        state = CompareState::Operator;
                    }
                    (_, region) => {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .visual(Pointer::new(self.lexer.source, region))
                            .help(format!(
                                "expected `{}`, `{}` or end of block",
                                Operator::And,
                                Operator::Or
                            )))
                    }
                },
            }
        }

        Ok(compare)
    }

    /// Parse a [`Keyword`].
    ///
    /// # Errors
    ///
    /// Returns an error if the next token is not a `Keyword`.
    fn parse_keyword(&mut self) -> Result<(Keyword, Region), Error> {
        match self.next_any_must()? {
            (Token::Keyword(keyword), region) => Ok((keyword, region)),
            (token, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .help(expected_keyword(token))
                .visual(Pointer::new(self.lexer.source, region))),
        }
    }

    /// Parse an [`Arguments`].
    ///
    /// If no arguments exist, [`None`] is returned instead.
    ///
    /// # Errors
    ///
    /// Propagates any errors that occur while parsing a [`Base`]
    /// for the argument(s).
    fn parse_args(&mut self) -> Result<Option<Arguments>, Error> {
        let mut values: Vec<(Option<Region>, Base)> = vec![];

        while !self.next_is(Token::Pipe)? && !self.next_is(Token::EndExpression)? {
            let name_or_value = self.parse_negated_base()?;
            if self.next_is(Token::Colon)? {
                self.next_must(Token::Colon)?;

                if name_or_value.get_negate() {
                    return Err(Error::build(INVALID_SYNTAX)
                        .visual(Pointer::new(self.lexer.source, name_or_value.get_region()))
                        .help(
                            "you might have tried to negate the argument name rather than \
                            the argument value",
                        ));
                }

                let value = self.parse_negated_base()?;
                values.push((Some(name_or_value.get_region()), value))
            } else {
                values.push((None, name_or_value))
            }
        }
        if values.is_empty() {
            return Ok(None);
        }

        // TODO: Move this to impl?
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

    /// Parse an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Propagates an error from next_must if the next token is not an
    /// [`Identifier`].
    fn parse_ident(&mut self) -> Result<Identifier, Error> {
        let (_, region) = self.next_must(Token::Identifier)?;
        Ok(Identifier { region })
    }

    /// Parse a [`Base`].
    ///
    /// A `Base` may be "negated", but this function is only responsible for
    /// parsing the `Base` itself, and so returns a `Base` with the "negate"
    /// property set to false.
    ///
    /// To parse a `Base` while checking for a negation operator, instead use
    /// [`parse_base_negated`].
    fn parse_base(&mut self) -> Result<Base, Error> {
        let expression = match self.next_any_must()? {
            // `Keyword` is valid as a `Base`, but only if it ends up
            // being a boolean literal.
            (Token::Keyword(keyword), region) => {
                let literal = self
                    .parse_bool_literal(region)
                    // Set a more appropriate error message if this fails.
                    .map_err(|e| {
                        e.help(format!("only `true` or `false` boolean literal keywords are valid here, found `{}`", keyword))
                    })?;

                Base::Literal(literal)
            }
            (Token::Operator(operator), region) => match operator {
                Operator::Add | Operator::Subtract => {
                    let (_, next_region) = self.next_must(Token::Number)?;

                    // -1000 | +1000  <- valid, negative/positive numbers
                    // - 1000 | + 1000<- invalid
                    if !region.is_neighbor(next_region) {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .visual(Pointer::new(self.lexer.source, region.combine(next_region)))
                            .help(format!(
                                "you can use `{}` to make `{}` a negative number, \
                                but you must remove the separating whitespace",
                                Operator::Subtract,
                                &self.lexer.source[next_region],
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
                            "only `{}` or `{}` operators to indicate a positive or negative \
                            numbers are valid here",
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
                Base::Variable(Variable::new(path))
            }
            (token, region) => {
                println!("{}", token);
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .visual(Pointer::new(self.lexer.source, region))
                    .help(format!(
                        "expected a variable or identifier, found `{}`",
                        token
                    )));
            }
        };

        Ok(expression)
    }

    /// Parse a [`Literal`] containing a [`Value::String`] from the literal value
    /// of the given [`Region`].
    ///
    /// # Errors
    ///
    /// Propagates any errors that occur while parsing a literal string,
    /// which may be caused by an unrecognized escape character.
    fn parse_string_literal(&mut self, region: Region) -> Result<Literal, Error> {
        let value = Value::String(self.parse_string(region)?);
        Ok(Literal {
            value,
            region,
            negate: None,
        })
    }

    /// Parse a [`Key`].
    ///
    /// # Errors
    ///
    /// Returns an error if the next token is not a valid [`Identifier`].
    fn parse_key(&mut self) -> Result<Key, Error> {
        match self.next_any_must()? {
            (Token::Identifier, region) => Ok(Key::from(Identifier { region })),
            (_, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .visual(Pointer::new(self.lexer.source, region))
                .help("expected an unquoted identifier such as `one.two`")),
        }
    }

    /// Parse a [`String`] from the literal value of the given [`Region`].
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

    /// Parse a [`Literal`] containing a [`Value::Number`] from the given [`Region`].
    ///
    /// # Errors
    ///
    /// Returns an error if the literal value of the `Region` cannot be converted
    /// to a [`Value::Number`].
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

        Ok(Literal::new(Value::Number(as_number), region))
    }

    /// Return a [`Literal`] containing a [`Value::Bool`] from the [`Region`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `Region` does not point to a boolean literal.
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
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .visual(Pointer::new(self.lexer.source, region))
                    .help("expected `true` or `false` boolean literal"))
            }
        };

        Ok(Literal {
            value: Value::Bool(bool),
            region,
            negate: None,
        })
    }

    /// Peek the next [`Token`].
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the underlying Lexer.
    fn peek(&mut self) -> LexResult {
        if let o @ None = &mut self.buffer {
            *o = Some(self.lexer.next()?);
        }

        Ok(self.buffer.unwrap())
    }

    /// Peek the next [`Token`].
    ///
    /// # Errors
    ///
    /// Returns an error if the [`Lexer`] returns [`None`], or when the `Lexer` returns
    /// an error itself.
    fn peek_must(&mut self) -> LexResultMust {
        let peek = self.peek()?;
        if peek.is_none() {
            return Err(unexpected_eof(self.lexer.source));
        }

        Ok(peek.unwrap())
    }

    /// Get the next [`Token`].
    ///
    /// Prefers to pull a `Token` from the internal buffer first, but will pull from
    /// the [`Lexer`] when the buffer is empty.
    fn next(&mut self) -> LexResult {
        match self.buffer.take() {
            Some(t) => Ok(t),
            None => self.lexer.next(),
        }
    }

    /// Returns true if the given [`Token`] matches the upcoming token.
    ///
    /// # Errors
    ///
    /// Propagates any errors reported by the underlying [`Lexer`].
    fn next_is(&mut self, expect: Token) -> Result<bool, Error> {
        Ok(self
            .peek()?
            .map(|(token, _)| token == expect)
            .unwrap_or(false))
    }

    /// Get the next [`Token`], and compare it to the given `Token`.
    ///
    /// # Errors
    ///
    /// Returns an error if the next `Token` does not match the given `Token`,
    /// or none are left.
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

    /// Get the next [`Token`].
    ///
    /// Similar to [`next`][`Parser::next`] but requires that a token is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if no tokens are left.
    fn next_any_must(&mut self) -> LexResultMust {
        match self.next()? {
            Some((token, region)) => Ok((token, region)),
            None => Err(unexpected_eof(self.lexer.source)),
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
        let source = "hello (( name | prepend 1: \"hello, \" | append \"!\" | upper ))";
        let result = Parser::new(source).compile();
        assert!(result.is_ok());
        // println!("{:#?}", result.unwrap().scope.tokens);
        // println!("{}", text.get(6..60).unwrap())
    }

    #[test]
    fn test_parse_negative_num_err() {
        let source = "balance: (( - 1000 ))";
        let result = Parser::new(source).compile();
        assert!(result.is_err(),);
    }

    #[test]
    fn test_peek_multiple() {
        let source = "(( one two";
        let mut parser = Parser::new(source);
        assert!(parser.next().is_ok());
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
    }

    #[test]
    fn test_parse_compare_valid() {
        //                                                   |-| is_admin negated here
        let source = "(* if this >= that && these == those || !is_admin *)";
        //                  ------------    --------------    ---------
        //                   Check 1     +     Check 2         Check 1
        //                  ------------------------------    ---------
        //                             Path 1                  Path 2
        let mut parser = get_parser_n(source, 2);
        let mut result = parser.parse_compare().unwrap();
        assert_eq!(result.paths.len(), 2);
        assert_eq!(result.paths.first().unwrap().len(), 2);
        assert_eq!(result.paths.last().unwrap().len(), 1);

        let is_admin_left = &result.last_check_mut_must("").left;
        // The underlying `Region` only spans the actual `Base`, which is "is_admin"
        // here, but get_region() is aware of negation and will return a new `Region`
        // that includes the "!".
        assert_eq!(
            is_admin_left.get_region().literal(source).unwrap(),
            "!is_admin"
        );
    }

    #[test]
    fn test_parse_compare_missing_base() {
        let source = "(* if this >= *)";
        //                         ^-- expected `Base` here
        let mut parser = get_parser_n(source, 2);

        let result = parser.parse_compare();
        assert!(result.is_err());
        println!("{:#}", result.unwrap_err());
    }

    #[test]
    fn test_parse_compare_bad_operator() {
        let source = "(* if this = that *)";
        //                       ^-- did you mean `==`?
        let mut parser = get_parser_n(source, 2);

        let result = parser.parse_compare();
        assert!(result.is_err());
        // println!("{:#}", result.unwrap_err());
    }

    /// Return a [`Parser`] over the given text that has already read
    /// "n" amount of tokens.
    ///
    /// # Panics
    ///
    /// Panics if any of the first `n` tokens in the given source cause
    /// the parser to return an error,
    fn get_parser_n(source: &str, n: i8) -> Parser {
        let mut parser = Parser::new(source);
        for _ in 0..n {
            parser.next().unwrap();
        }

        parser
    }
}
