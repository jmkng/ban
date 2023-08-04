pub mod scope;
pub mod tree;

mod block;
mod state;

use crate::{
    compile::{
        lex::{token::Token, LexResult, LexResultMust, Lexer},
        parse::{
            block::Block,
            state::{BlockState, CheckState},
            tree::*,
        },
        Keyword, Operator, Scope, Template,
    },
    log::{
        message::{
            error_eof, expected_operator, INVALID_SYNTAX, UNEXPECTED_BLOCK, UNEXPECTED_EOF,
            UNEXPECTED_TOKEN,
        },
        Error,
    },
    region::Region,
};
use serde_json::{Number, Value};

/// Provides methods to transform an input stream of [`Token`] into an abstract
/// syntax tree composed of [`Tree`].
pub struct Parser<'source> {
    /// `Lexer` used to pull from source as tokens instead of raw text.
    lexer: Lexer<'source>,
    /// Store peeked tokens.
    ///
    /// Double option is used to remember when the next token is None.
    buffer: Option<Option<(Token, Region)>>,
    /// Temporarily store an [`Extends`] for the [`Template`] that is being
    /// parsed.
    extended: Option<Extends>,
}

impl<'source> Parser<'source> {
    /// Create a new `Parser` from the given string.
    #[inline]
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: Lexer::new(source),
            buffer: None,
            extended: None,
        }
    }

    /// Compile a [`Template`].
    ///
    /// Returns a new `Template`, which can be executed with some [`Store`][`crate::Store`]
    /// data to receive output.
    pub fn compile(mut self, name: Option<String>) -> Result<Template, Error> {
        // Temporary storage for fragments of larger blocks.
        let mut states: Vec<BlockState> = vec![];

        // Storage for a stack of [`Scope`] instances.
        //
        // The first, top level `Scope` will be used to create the `Template`.
        //
        // Additional scopes are pushed when blocks like "if" and "for" are started,
        // in order to capture the body of those blocks.
        //
        // When a closing block such as "endif" or "endfor" is seen, the scopes are
        // popped off and used to create an appropriate [`Tree`] instance that is
        // then pushed to the first `Scope`.
        let mut scopes: Vec<Scope> = vec![Scope::new()];

        while let Some(next) = self.next()? {
            let tree = match next {
                // Raw text.
                //
                // Expected:
                //
                // ...
                (Token::Raw, region) => Tree::Raw(region),
                // Beginning of an expression.
                //
                // Expected:
                //
                // BASE [FILTER ...] ...
                (Token::BeginExpression, region) => {
                    let expression = self.parse_expression()?;
                    let end = self.next_must(Token::EndExpression)?.1.combine(region);
                    Tree::Output(Output::from((expression, end)))
                }
                // Beginning of a block.
                //
                // Expected:
                //
                // KEYWORD ...
                (Token::BeginBlock, region) => {
                    let block = self.parse_block()?;
                    let end = self.next_must(Token::EndBlock)?.1.combine(region);

                    match block {
                        // Beginning of an "if" block.
                        //
                        // Expected:
                        //
                        // [ELSEIF ...] | [ELSE] | ENDIF ...
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
                        // [ELSEIF ...] | [ELSE] | ENDIF ...
                        Block::ElseIf(tr) => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help("expected `if` before `else if`")
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
                        // ENDIF ...
                        Block::Else => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help("expected `if` before `else`")
                            };

                            match states.last_mut().ok_or_else(error)? {
                                BlockState::If {
                                    has_else: has_else @ false,
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
                                    .help("expected `if` before `endif`")
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
                        // BASE [,] [BASE] IN BASE ENDFOR ...
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
                                    .help("expected `for` before `endfor`")
                            };

                            let tree = match states.pop().ok_or_else(error)? {
                                BlockState::For { set, base, .. } => {
                                    let scope = scopes.pop().unwrap();
                                    Tree::For(Iterable {
                                        set,
                                        base,
                                        scope,
                                        region,
                                    })
                                }
                                _ => return Err(error()),
                            };

                            tree
                        }
                        // A "let" block.
                        //
                        // Expected:
                        //
                        // BASE ASSIGN BASE ...
                        Block::Let(left, right) => Tree::Let(Let { left, right }),
                        // An "include" block.
                        //
                        // Expected:
                        //
                        // BASE [BASE] ...
                        Block::Include(name, mount) => Tree::Include(Include { name, mount }),
                        // An "extends" block.
                        //
                        // Expected:
                        //
                        // ...
                        Block::Extends(name) => {
                            if scopes.len() != 1 || !scopes.first().unwrap().data.is_empty() {
                                return Err(Error::build(UNEXPECTED_BLOCK)
                                    .pointer(&self.lexer.source, end)
                                    .help("block `extend` must appear at top of template"));
                            }

                            self.extended = Some(Extends { name, region: end });
                            continue;
                        }
                        // A "block" block.
                        //
                        // Expected:
                        //
                        // ENDBLOCK ...
                        Block::Block(name) => {
                            states.push(BlockState::Block { name, region: end });
                            scopes.push(Scope::new());
                            continue;
                        }
                        // An "endblock" block.
                        //
                        // Expected:
                        //
                        // ...
                        Block::EndBlock => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .pointer(self.lexer.source, end)
                                    .help("expected `block` before `endblock`")
                            };

                            let previous = states.pop().ok_or_else(error)?;
                            match previous {
                                BlockState::Block { name, region } => Tree::Block(tree::Block {
                                    name,
                                    scope: scopes.pop().unwrap(),
                                    region: end.combine(region),
                                }),
                                _ => return Err(error()),
                            }
                        }
                    }
                }
                //
                _ => unreachable!("lexer will abort without begin block"),
            };

            scopes.last_mut().unwrap().data.push(tree);
        }

        if let Some(block) = states.first() {
            let (block, close, region) = match block {
                BlockState::If { region, .. } => ("if", "endif", region),
                BlockState::For { region, .. } => ("for", "endfor", region),
                BlockState::Block { region, .. } => ("block", "endblock", region),
            };
            return Err(Error::build(INVALID_SYNTAX)
                .pointer(self.lexer.source, *region)
                .help(format!("did you close the `{block}` with `{close}`?")));
        }

        assert!(scopes.len() == 1, "must have single scope");
        Ok(Template {
            name,
            scope: scopes.remove(0),
            source: self.lexer.source.to_owned(),
            extended: self.extended,
        })
    }

    /// Parse a [`Block`].
    ///
    /// A `Block` is a call to evaluate some kind of expression which may have
    /// side effects on the [`Shadow`][`crate::store::Shadow`] data.
    fn parse_block(&mut self) -> Result<Block, Error> {
        //   from
        //   |
        // (* if name == "taylor" *)
        //   Welcome back, Taylor.
        // (* endfor *)
        //          |
        //          to
        let (keyword, region) = self.parse_keyword()?;

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
            Keyword::Include => {
                let name = self.parse_base()?;
                let scope = self.parse_mount()?;
                Ok(Block::Include(name, scope))
            }
            Keyword::Extends => {
                let name = self.parse_base()?;
                Ok(Block::Extends(name))
            }
            Keyword::Block => {
                let name = self.parse_base()?;
                Ok(Block::Block(name))
            }
            Keyword::EndBlock => Ok(Block::EndBlock),
            k @ Keyword::Not | k @ Keyword::In => Err(Error::build(UNEXPECTED_TOKEN)
                .pointer(self.lexer.source, region)
                .help(format!("keyword `{k}` is not valid in this position"))),
        }
    }

    /// Parse a [`Mount`].
    ///
    /// Similar to `.parse_arguments`, but requires that all arguments
    /// are named.
    fn parse_mount(&mut self) -> Result<Option<Mount>, Error> {
        // "(* include "header" title: site.title *)"
        //                     ^-----------------^
        //                     from              to
        let mut values: Vec<Point> = vec![];

        while !self.peek_is(Token::EndBlock)? {
            let argument = self.parse_argument()?;
            if argument.name.is_none() {
                return Err(Error::build(INVALID_SYNTAX)
                    .pointer(self.lexer.source, argument.value.get_region())
                    .help(format!(
                        "arguments in an `include` block must be named, \
                        try `[name: ]{}`",
                        argument.value.get_region().literal(self.lexer.source)
                    )));
            }

            values.push(Point {
                name: argument.name.unwrap(),
                value: argument.value,
            })
        }
        if values.is_empty() {
            return Ok(None);
        }

        let first = values.first().unwrap();
        let mut region = first.name.combine(first.value.get_region());
        if values.len() > 1 {
            let last = values.last().unwrap();
            region = region.combine(last.value.get_region())
        }

        Ok(Some(Mount { values, region }))
    }

    /// Parse an [`Expression`].
    ///
    /// An `Expression` is a call to render some kind of data,
    /// and may contain one or more "filters" which are used to modify the output.
    fn parse_expression(&mut self) -> Result<Expression, Error> {
        // (( name | prepend 1: "hello, " | append "!" | upper ))
        //   |                                                |
        //   from                                             to
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
                .help(format!(
                    "expected keyword like `if`, `else`, `endif`, `let`, `for`, `in`, `endfor`, `include`, \
                    `extends`, `block`, `endblock`, found `{token}`"
                ))
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
        // "hello (( name | prepend 1: "hello, " | append "!", "?" | upper ))"
        //                         ^------------^        ^--------^
        //                         from         to       from     to
        let mut values: Vec<Argument> = vec![];

        while !self.peek_is(Token::Pipe)? && !self.peek_is(Token::EndExpression)? {
            values.push(self.parse_argument()?);
        }
        if values.is_empty() {
            return Ok(None);
        }

        let mut region = values.first().unwrap().get_region();
        if values.len() > 1 {
            let last = values.last().unwrap();
            region = region.combine(last.get_region())
        }

        Ok(Some(Arguments { values, region }))
    }

    /// Parse an [`Argument`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a valid `Argument` cannot be made
    /// with the next tokens.
    fn parse_argument(&mut self) -> Result<Argument, Error> {
        let name_or_value = self.parse_base()?;

        // Named argument.
        if self.peek_is(Token::Colon)? {
            self.next_must(Token::Colon)?;

            let value = self.parse_base()?;
            return Ok(Argument {
                name: Some(name_or_value.get_region()),
                value,
            });
        }

        // Anonymous argument.
        Ok(Argument {
            name: None,
            value: name_or_value,
        })
    }

    /// Parse an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the next token is not an [`Identifier`].
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
                                "remove the separating whitespace to make `{}` a negative number",
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
                let mut path = vec![Identifier { region }];

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
    fn parse_key(&mut self) -> Result<Identifier, Error> {
        match self.next_any_must()? {
            (Token::Identifier, region) => Ok(Identifier { region }),
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
        let window = region.literal(self.lexer.source);
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
    use super::{tree::Tree, Parser};
    use crate::compile::{lex::token::Token, tree::Set};

    #[test]
    fn test_parser_lexer_integration() {
        let mut parser = Parser::new("hello");

        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }

    #[test]
    fn test_parse_full_expression() {
        let source = "hello (( name | prepend text: \"hello, \" | append \"!\" \"?\" | upper ))";
        let result = Parser::new(source).compile(None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_negative_num_err() {
        let source = "balance: (( - 1000 ))";
        //                         ^-- remove whitespace for negative num

        assert!(Parser::new(source).compile(None).is_err());
    }

    #[test]
    fn test_peek_multiple() {
        let mut parser = Parser::new("(( one two");

        // Multiple calls to peek should return the same thing.
        assert!(parser.next().is_ok());
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
    }

    #[test]
    fn test_parse_tree_valid() {
        //                                                    --- negated
        let source = "(* if this >= that && these == those || not is_admin *)";
        //                  ------------    --------------    ------------
        //                     Check 1          Check 2          Check 1
        //                  ------------------------------    ------------
        //                             Branch 1                  Branch 2
        //                  ----------------------------------------------
        //                                         Tree
        let result = get_parser_n(source, 2).parse_tree().unwrap();

        assert_eq!(result.branches.len(), 2);
        assert_eq!(result.branches.first().unwrap().len(), 2);
        assert_eq!(result.branches.last().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_tree_missing_base() {
        let mut parser = get_parser_n("(* if this >= *)", 2);
        //                                          ^-- expected `Base` here

        assert!(parser.parse_tree().is_err());
    }

    #[test]
    fn test_parse_tree_bad_operator() {
        let mut parser = get_parser_n("(* if this = that *)", 2);
        //                                        ^-- did you mean `==`?

        assert!(parser.parse_tree().is_err());
    }

    #[test]
    fn test_parse_set_pair() {
        //                         ---- identifier 2
        let source = "(* for this, that in thing *)hello(* endfor *)";
        //      identifier 1 ----     base -----   ----- scope
        let result = get_parser_n(source, 2).parse_set().unwrap();

        match result {
            Set::Pair(pa) => {
                assert_eq!(pa.key.region.literal(source), "this");
                assert_eq!(pa.value.region.literal(source), "that");
                assert_eq!(pa.region.literal(source), "this, that");
            }
            _ => panic!("two variables should create a pair"),
        }
    }

    #[test]
    fn test_parse_mount() {
        //                                    -------- name
        let result = get_parser_n("(* include \"base\" x: data.related *)", 3).parse_mount();
        //                                             --------------- mount

        assert!(result.is_ok_and(|t| t.is_some()));
    }

    #[test]
    fn test_parse_block() {
        //                     ---- name
        let source = "(* block main *)abc(* endblock *)def";
        let template = get_parser_n(source, 0).compile(None).unwrap();

        match template.scope.data.first().unwrap() {
            Tree::Block(block) => {
                assert_eq!(block.name.get_region().literal(source), "main");
                match block.scope.data.first().unwrap() {
                    Tree::Raw(raw) => assert_eq!(raw.literal(source), "abc"),
                    _ => panic!("unexpected block scope"),
                }
            }
            _ => panic!("expected block"),
        }
        match template.scope.data.last().unwrap() {
            Tree::Raw(raw) => assert_eq!(raw.literal(source), "def"),
            unexpected => panic!("expected raw text `def`, found `{:?}`", unexpected),
        }
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
