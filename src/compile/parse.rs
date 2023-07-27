pub mod scope;
pub mod tree;

mod block;
mod state;

use self::state::CheckState;
use crate::{
    compile::{
        lex::{token::Token, LexResult, LexResultMust, Lexer},
        parse::{block::Block, state::BlockState, tree::*},
        Keyword, Operator, Scope, Template,
    },
    log::{
        error_eof, expected_keyword, expected_operator, Error, INVALID_SYNTAX, UNEXPECTED_BLOCK,
        UNEXPECTED_EOF, UNEXPECTED_TOKEN,
    },
    region::Region,
};
use serde_json::{Number, Value};

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
    pub fn compile(mut self, name: Option<&'source str>) -> Result<Template<'source>, Error> {
        // Temporary storage for fragments of larger blocks.
        let mut states: Vec<BlockState> = vec![];
        // Storage for a stack of [`Scope`] instances.
        //
        // When a closing block such as "endif" is seen, they are rolled into a
        // [`Tree`] instance for the top level scope.
        let mut scopes: Vec<Scope> = vec![Scope::new()];

        while let Some(next) = self.next()? {
            let tree = match next {
                (Token::Raw, region) => Tree::Raw(region),
                // Beginning of an expression.
                //
                // Expected:
                //
                // (BASE) [FILTER ...]
                // ------
                (Token::BeginExpression, region) => {
                    let expression = self.parse_expression()?;
                    let end = self.next_must(Token::EndExpression)?.1.combine(region);
                    Tree::Output(Output::from((expression, end)))
                }
                // Beginning of a block.
                //
                // Expected:
                //
                // (KEYWORD)
                // ---------
                (Token::BeginBlock, region) => {
                    let block = self.parse_block()?;
                    let end = self.next_must(Token::EndBlock)?.1.combine(region);

                    match block {
                        // Beginning of an "if" block.
                        //
                        // Expected:
                        //
                        // [ELSEIF ...] | [ELSE] | ENDIF
                        //                         -----
                        Block::If(tr) => {
                            states.push(BlockState::If {
                                else_if: false,
                                tree: tr,
                                region: end,
                                has_else: false,
                            });
                            scopes.push(Scope::new());
                            continue;
                        }
                        // The "else if" fragment of an "if" block.
                        //
                        // Expected:
                        //
                        // [ELSEIF ...] | [ELSE] | ENDIF
                        //                         -----
                        Block::ElseIf(tr) => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help(
                                        "unexpected `else if` block, did you forget to write \
                                        an `if` block first?",
                                    )
                            };

                            match states.last_mut().ok_or_else(error)? {
                                BlockState::If {
                                    has_else: has_else @ false,
                                    ..
                                } => *has_else = true,
                                _ => return Err(error()),
                            }

                            states.push(BlockState::If {
                                else_if: true,
                                tree: tr,
                                region: end,
                                has_else: false,
                            });

                            scopes.push(Scope::new());
                            scopes.push(Scope::new());
                            continue;
                        }
                        // The "else" fragment of an "if" block.
                        //
                        // Expected:
                        //
                        // ENDIF
                        // -----
                        Block::Else => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help(
                                        "unexpected `else` block, did you forget to write an \
                                        `if` block first?",
                                    )
                            };

                            match states.last_mut().ok_or_else(error)? {
                                BlockState::If {
                                    has_else: has_else @ false,
                                    region: end,
                                    ..
                                } => *has_else = true,
                                _ => return Err(error()),
                            }

                            scopes.push(Scope::new());
                            continue;
                        }
                        // End of an "if" block.
                        //
                        // Expected:
                        //
                        // ...
                        Block::EndIf => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help(
                                        "unexpected end of `if` block, did you forget to write \
                                        an `if` block first?",
                                    )
                            };

                            loop {
                                match states.pop().ok_or_else(error)? {
                                    BlockState::If {
                                        else_if,
                                        tree,
                                        has_else,
                                        region,
                                        ..
                                    } => {
                                        let else_branch = has_else.then(|| scopes.pop().unwrap());
                                        let then_branch = scopes.pop().unwrap();

                                        let tree = Tree::If(If {
                                            tree,
                                            then_branch,
                                            else_branch,
                                            region: end.combine(region),
                                        });
                                        if !else_if {
                                            break tree;
                                        }
                                        scopes.last_mut().unwrap().data.push(tree);
                                    }
                                    _ => return Err(error()),
                                }
                            }
                        }
                        // Beginning of an "for" block.
                        //
                        // Expected:
                        //
                        // BASE [,] [BASE] IN BASE
                        // ----            -- ----
                        Block::For(set, base) => {
                            states.push(BlockState::For { set, base, region });
                            scopes.push(Scope::new());
                            continue;
                        }
                        // End of a "for" block.
                        //
                        // Expected:
                        //
                        // ...
                        Block::EndFor => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help(
                                        "unexpected end of `for` block, did you forget to write a \
                                        `for` block first?",
                                    )
                            };

                            let tree = match states.pop().ok_or_else(error)? {
                                BlockState::For { set, base, .. } => {
                                    let data = scopes.pop().unwrap();
                                    Tree::For(Iterable {
                                        set,
                                        base,
                                        data,
                                        region,
                                    })
                                }
                                _ => return Err(error()),
                            };

                            tree
                        }
                        Block::Let(identifier, base) => Tree::Let(Let {
                            left: identifier,
                            right: base,
                        }),
                        // TODO
                        Block::Include(_, _) => todo!(),
                    }
                }
                _ => unreachable!("lexer will abort without begin block"),
            };

            scopes.last_mut().unwrap().data.push(tree);
        }

        if let Some(block) = states.first() {
            let (block, close, region) = match block {
                BlockState::If { region, .. } => ("if", "endif", region),
                BlockState::For { region, .. } => ("for", "endfor", region),
            };

            return Err(Error::build(INVALID_SYNTAX)
                .pointer(self.lexer.source, *region)
                .help(format!(
                    "did you close the `{block}` block with a `{close}` block?"
                )));
        }

        assert!(scopes.len() == 1, "must have single scope");
        Ok(Template {
            name,
            scope: scopes.remove(0),
            source: self.lexer.source,
        })
    }

    /// Parse a [`Block`].
    ///
    /// A `Block` is a call to evaluate some kind of expression which may have
    /// side effects on the `Shadow` data.
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
                let tree = self.parse_tree()?;
                Ok(Block::If(tree))
            }
            Keyword::Else => {
                if self.peek_is(Token::Keyword(Keyword::If))? {
                    self.next_must(Token::Keyword(Keyword::If))?;
                    let tree = self.parse_tree()?;
                    Ok(Block::ElseIf(tree))
                } else {
                    Ok(Block::Else)
                }
            }
            Keyword::EndIf => Ok(Block::EndIf),
            Keyword::For => {
                let variables = self.parse_set()?;
                self.next_must(Token::Keyword(Keyword::In))?;
                let base = self.parse_base()?;
                Ok(Block::For(variables, base))
            }
            Keyword::EndFor => Ok(Block::EndFor),
            Keyword::Let => {
                let left = self.parse_identifier()?;
                self.next_must(Token::Assign)?;
                let right = self.parse_base()?;
                Ok(Block::Let(left, right))
            }
            // TODO
            Keyword::Include => todo!(),
            // TODO
            Keyword::Extends => todo!(),
            // TODO
            Keyword::Block => todo!(),
            // TODO
            Keyword::EndBlock => todo!(),
            // TODO
            _ => todo!(), // TODO: err
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

        while self.peek_is(Token::Pipe)? {
            self.next_must(Token::Pipe)?;

            let name = self.parse_identifier()?;
            let arguments = self.parse_arguments()?;
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

    /// Parse a [`CheckTree`].
    ///
    /// This `CheckTree` will contain all of the information necessary
    /// to determine if the block should pass.
    fn parse_tree(&mut self) -> Result<CheckTree, Error> {
        // this >= that && these == those || a == b *)
        // |-----------    --------------    ------|
        // from  |                  |            |  to
        //       negatable          negatable    negatable
        let mut tree = CheckTree::new();
        let mut state = CheckState::default();

        loop {
            match state {
                // Parse a `Base` and assign it to either "left" or "right"
                // on the latest `Check` instance.
                CheckState::Base(has_left) => {
                    if has_left {
                        let base = self.parse_base()?;
                        tree.last_check_mut_must("has_left implies that a check exists")
                            .right = Some(base);

                        state = CheckState::Transition;
                    } else {
                        let (base, negated) = if self.peek_is(Token::Keyword(Keyword::Not))? {
                            self.next_must(Token::Keyword(Keyword::Not))?;
                            (self.parse_base()?, true)
                        } else {
                            (self.parse_base()?, false)
                        };
                        tree.last_mut_must().push(Check::new(base, negated));

                        state = CheckState::Operator;
                    };
                }
                // Parse an `Operator` for the `Check`.
                //
                // Set state to CompareState::Transition if something like "&&", "||"
                // or Token::EndBlock shows up.
                CheckState::Operator => match self.peek_must()? {
                    (token, region) => match token {
                        Token::EndBlock | Token::Or | Token::And => state = CheckState::Transition,
                        Token::Operator(op) => {
                            self.next_must(Token::Operator(op))?;
                            tree.last_check_mut_must("operator state implies that check exists")
                                .operator = Some(op);
                            state = CheckState::Base(true);
                        }
                        unexpected => {
                            return Err(Error::build(UNEXPECTED_TOKEN)
                                .pointer(self.lexer.source, region)
                                .help(expected_operator(unexpected)))
                        }
                    },
                },
                // Handle "&&", "||" and `Token::EndBlock`.
                CheckState::Transition => match self.peek_must()? {
                    (Token::EndBlock, _) => break,
                    (Token::Or, _) => {
                        self.next_must(Token::Or)?;
                        // Split up the `Path`.
                        tree.split_branch();
                        state = CheckState::Base(false);
                    }
                    (Token::And, _) => {
                        self.next_must(Token::And)?;
                        // Start a new `Check`, which requires a `Base`.
                        let (base, negated) = if self.peek_is(Token::Keyword(Keyword::Not))? {
                            self.next_must(Token::Keyword(Keyword::Not))?;
                            (self.parse_base()?, true)
                        } else {
                            (self.parse_base()?, false)
                        };
                        tree.split_check(Check::new(base, negated));
                        state = CheckState::Operator;
                    }
                    unexpected => {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .pointer(self.lexer.source, unexpected.1)
                            .help(format!(
                                "expected `{}`, `{}` or end of block, found `{}`",
                                Token::And,
                                Token::Or,
                                unexpected.0
                            )))
                    }
                },
            }
        }

        Ok(tree)
    }

    /// Parse a [`Set`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the next token is not an [`Identifier`].
    fn parse_set(&mut self) -> Result<Set, Error> {
        let key = self.parse_identifier()?;
        if !self.peek_is(Token::Comma)? {
            return Ok(Set::Single(key));
        }

        self.next_must(Token::Comma)?;
        let value = self.parse_identifier()?;
        let merged = key.region.combine(value.region);

        Ok(Set::Pair(KeyValue {
            key,
            value,
            region: merged,
        }))
    }

    /// Parse a [`Keyword`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the next token is not a [`Keyword`].
    fn parse_keyword(&mut self) -> Result<(Keyword, Region), Error> {
        match self.next_any_must()? {
            (Token::Keyword(keyword), region) => Ok((keyword, region)),
            (token, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .help(expected_keyword(token))
                .pointer(self.lexer.source, region)),
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
    fn parse_arguments(&mut self) -> Result<Option<Arguments>, Error> {
        let mut values: Vec<(Option<Region>, Base)> = vec![];

        while !self.peek_is(Token::Pipe)? && !self.peek_is(Token::EndExpression)? {
            let name_or_value = self.parse_base()?;
            if self.peek_is(Token::Colon)? {
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
    fn parse_identifier(&mut self) -> Result<Identifier, Error> {
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
            (Token::False, region) => Base::Literal(Literal {
                value: Value::Bool(false),
                region,
            }),
            (Token::True, region) => Base::Literal(Literal {
                value: Value::Bool(true),
                region,
            }),
            (Token::Operator(operator), region) => match operator {
                Operator::Add | Operator::Subtract => {
                    let (_, next_region) = self.next_must(Token::Number)?;

                    // -1000 | +1000  <- valid, negative/positive numbers
                    // - 1000 | + 1000<- invalid
                    if !region.is_neighbor(next_region) {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .pointer(self.lexer.source, region.combine(next_region))
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
                        .pointer(self.lexer.source, region)
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
                while self.peek_is(Token::Period)? {
                    self.next_must(Token::Period)?;
                    path.push(self.parse_key()?);
                }
                Base::Variable(Variable::new(path))
            }
            (token, region) => {
                println!("{}", token);
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .pointer(self.lexer.source, region)
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
        Ok(Literal { value, region })
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
                .pointer(self.lexer.source, region)
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
                                    .pointer(self.lexer.source, region))
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
                .pointer(self.lexer.source, region)
                .help(format!(
                    "numbers may begin with `{}` to indicate a negative \
                    number and must not end with a decimal",
                    Operator::Subtract
                ))
        })?;

        Ok(Literal::new(Value::Number(as_number), region))
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
            return Err(error_eof(self.lexer.source));
        }

        Ok(peek.unwrap())
    }

    /// Returns true if the next token matches the given [`Token`].
    ///
    /// # Errors
    ///
    /// Propagates any errors reported by the underlying [`Lexer`].
    fn peek_is(&mut self, expect: Token) -> Result<bool, Error> {
        Ok(self
            .peek()?
            .map(|(token, _)| token == expect)
            .unwrap_or(false))
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
                        .pointer(self.lexer.source, region)
                        .help(format!("expected `{expect}`")))
                }
            }
            None => {
                let source_len = self.lexer.source.len();
                Err(Error::build(UNEXPECTED_EOF)
                    .pointer(self.lexer.source, source_len..source_len)
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
            None => Err(error_eof(self.lexer.source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::compile::{lex::token::Token, tree::Set};

    #[test]
    fn test_parser_lexer_integration() {
        let mut parser = Parser::new("hello");
        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }

    #[test]
    fn test_parse_full_expression() {
        let source = "hello (( name | prepend 1: \"hello, \" | append \"!\" | upper ))";
        let result = Parser::new(source).compile(None);
        assert!(result.is_ok());
        // println!("{:#?}", result.unwrap().scope.tokens);
        // println!("{}", text.get(6..60).unwrap())
    }

    #[test]
    fn test_parse_negative_num_err() {
        let source = "balance: (( - 1000 ))";
        let result = Parser::new(source).compile(None);
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
    fn test_parse_tree_valid() {
        //                                                   |---| is_admin negated here
        let source = "(* if this >= that && these == those || not is_admin *)";
        //                  ------------    --------------    ------------
        //                     Check 1          Check 2          Check 1
        //                  ------------------------------    ------------
        //                             Branch 1                  Branch 2
        //                  ----------------------------------------------
        //                                        Tree
        let mut parser = get_parser_n(source, 2);
        let result = parser.parse_tree().unwrap();
        assert_eq!(result.branches.len(), 2);
        assert_eq!(result.branches.first().unwrap().len(), 2);
        assert_eq!(result.branches.last().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_tree_missing_base() {
        let source = "(* if this >= *)";
        //                         ^-- expected `Base` here
        let mut parser = get_parser_n(source, 2);

        let result = parser.parse_tree();
        assert!(result.is_err());
        // println!("{:#}", result.unwrap_err());
    }

    #[test]
    fn test_parse_tree_bad_operator() {
        let source = "(* if this = that *)";
        //                       ^-- did you mean `==`?
        let mut parser = get_parser_n(source, 2);

        let result = parser.parse_tree();
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

    #[test]
    fn test_parse_for_valid() {
        //                      Identifier 2
        //                         ----
        let source = "(* for this, that in thing *)hello(* endfor *)";
        //                   ----          -----   -----
        //                Identifier 1      Base   Data <---- rendered n times
        let mut parser = get_parser_n(source, 2);
        let result = parser.parse_set().unwrap();
        match result {
            Set::Pair(pa) => {
                assert_eq!(pa.key.region.literal(source).unwrap(), "this");
                assert_eq!(pa.value.region.literal(source).unwrap(), "that");
                assert_eq!(pa.region.literal(source).unwrap(), "this, that");
            }
            _ => unreachable!(),
        }
    }
}
