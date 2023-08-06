pub mod scope;
pub mod tree;

mod fragment;
mod state;

use crate::{log::Error, region::Region};

use super::{
    expected_operator,
    lex::{token::Token, Lexer},
    parse::{
        fragment::Fragment,
        state::{BlockState, IfState},
        tree::*,
    },
    Keyword, Operator, Scope, Template, TokenResult, TokenResultMust, INVALID_SYNTAX,
    UNEXPECTED_TOKEN,
};

use morel::Finder;
use serde_json::{Number, Value};

const UNEXPECTED_BLOCK: &str = "unexpected block";
const UNEXPECTED_EOF: &str = "unexpected eof";

/// Provides methods to transform an input stream of [`Token`] into an abstract
/// syntax tree composed of [`Tree`].
pub struct Parser<'source> {
    /// [`Lexer`] used to read [`Token`] instances from the source.
    lexer: Lexer<'source>,
    /// Store a peeked [`Token`] instance.
    buffer: Option<Option<(Token, Region)>>,
    /// Temporarily store an [`Extends`] for the [`Template`] that is being
    /// parsed.
    extended: Option<Extends>,
}

impl<'source> Parser<'source> {
    /// Create a new [`Parser`] from the given string and [`Syntax`].
    #[inline]
    pub fn new(source: &'source str, finder: &'source Finder) -> Self {
        Self {
            lexer: Lexer::new(source, finder),
            buffer: None,
            extended: None,
        }
    }

    /// Compile a [`Template`].
    ///
    /// Returns a new `Template`, which can be executed with some [`Store`][`crate::Store`]
    /// data to receive output.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when syntax leading to an invalid `Template` is found.
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
        // When a block is closed with "end", the scopes are popped off and used to create an appropriate
        // [`Tree`] instance to push to the first `Scope`.
        let mut scopes: Vec<Scope> = vec![Scope::new()];

        while let Some(next) = self.next()? {
            let tree = match next {
                (Token::Raw, region) => Tree::Raw(region),
                (Token::BeginExpression, region) => {
                    let expression = self.parse_expression()?;
                    let end = self.next_must(Token::EndExpression)?.1.combine(region);
                    Tree::Output(Output::from((expression, end)))
                }
                (Token::BeginBlock, region) => {
                    let fragment = self.parse_fragment()?;
                    let end = self.next_must(Token::EndBlock)?.1.combine(region);

                    match fragment {
                        Fragment::If(tr) => {
                            states.push(BlockState::If {
                                else_if: false,
                                tree: tr,
                                region: end,
                                has_else: false,
                            });
                            scopes.push(Scope::new());
                            continue;
                        }
                        Fragment::ElseIf(tr) => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .with_pointer(self.lexer.source, end)
                                    .with_help("expected `if` before `else if`")
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
                        Fragment::Else => {
                            let error = || {
                                Error::build(UNEXPECTED_BLOCK)
                                    .with_pointer(self.lexer.source, end)
                                    .with_help("expected `if` before `else`")
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
                        Fragment::For(set, base) => {
                            states.push(BlockState::For { set, base, region });
                            scopes.push(Scope::new());
                            continue;
                        }
                        Fragment::Let(left, right) => Tree::Let(Let { left, right }),
                        Fragment::Include(name, mount) => Tree::Include(Include { name, mount }),
                        Fragment::Extends(name) => {
                            if scopes.len() != 1 || !scopes.first().unwrap().data.is_empty() {
                                return Err(Error::build(UNEXPECTED_BLOCK)
                                    .with_pointer(&self.lexer.source, end)
                                    .with_help("block `extend` must appear at top of template"));
                            }

                            self.extended = Some(Extends { name, region: end });
                            continue;
                        }
                        Fragment::Block(name) => {
                            states.push(BlockState::Block { name, region: end });
                            scopes.push(Scope::new());
                            continue;
                        }
                        Fragment::End => match states.last() {
                            Some(parent) => match parent {
                                BlockState::If { .. } => loop {
                                    match states.pop().ok_or_else(|| Error::build(INVALID_SYNTAX)
                                                .with_pointer(self.lexer.source, end)
                                                .with_help("`if` block does not appear to have a beginning"))? {
                                            BlockState::If {
                                                else_if,
                                                tree,
                                                region,
                                                has_else,
                                            } => {
                                                let else_branch = has_else.then(|| scopes.pop().unwrap());
                                                let then_branch = scopes.pop().unwrap();
                                                let tree = Tree::If(If {
                                                    tree,
                                                    else_branch,
                                                    then_branch,
                                                    region: end.combine(region),
                                                });
                                                if !else_if {
                                                    break tree;
                                                }

                                                scopes.last_mut().unwrap().data.push(tree);
                                            }
                                            _ => return Err(Error::build(INVALID_SYNTAX)
                                                .with_pointer(self.lexer.source, end)
                                                .with_help("expected `if` expressions above this point, \
                                                    did you attempt to start a different block before closing this `if`?")),
                                        }
                                },
                                BlockState::For { .. } => match states.pop().unwrap() {
                                    BlockState::For { set, base, region } => Tree::For(For {
                                        set,
                                        base,
                                        scope: scopes.pop().unwrap(),
                                        region: end.combine(region),
                                    }),
                                    _ => unreachable!(),
                                },
                                BlockState::Block { .. } => match states.pop().unwrap() {
                                    BlockState::Block { name, region } => Tree::Block(Block {
                                        name,
                                        scope: scopes.pop().unwrap(),
                                        region: end.combine(region),
                                    }),
                                    _ => unreachable!(),
                                },
                            },
                            None => {
                                return Err(Error::build(
                                    "unexpected `end` expression, you must open a block first.",
                                ))
                            }
                        },
                    }
                }
                _ => unreachable!("lexer must abort without begin block"),
            };

            scopes.last_mut().unwrap().data.push(tree);
        }

        if let Some(block) = states.first() {
            let (block, region) = match block {
                BlockState::If { region, .. } => ("if", region),
                BlockState::For { region, .. } => ("for", region),
                BlockState::Block { region, .. } => ("block", region),
            };

            return Err(Error::build(INVALID_SYNTAX)
                .with_pointer(self.lexer.source, *region)
                .with_help(format!("did you close the `{block}` with `end`?")));
        }
        assert!(scopes.len() == 1, "must have single scope");

        Ok(Template::new(
            name,
            scopes.remove(0),
            self.lexer.source.to_owned(),
            self.extended,
        ))
    }

    /// Parse a [`Fragment`].
    ///
    /// A `Fragment` may encompass an entire expression, such as the "extends" expression,
    /// or may represent a smaller part of a larger block.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
    fn parse_fragment(&mut self) -> Result<Fragment, Error> {
        //   from                to
        //   |                   |
        // (* if name == "taylor" *)
        let (keyword, region) = self.parse_keyword()?;

        match keyword {
            Keyword::If => {
                let tree = self.parse_tree()?;
                Ok(Fragment::If(tree))
            }
            Keyword::Else => {
                if self.peek_is(Token::Keyword(Keyword::If))? {
                    self.next_must(Token::Keyword(Keyword::If))?;
                    let tree = self.parse_tree()?;
                    Ok(Fragment::ElseIf(tree))
                } else {
                    Ok(Fragment::Else)
                }
            }
            Keyword::For => {
                let variables = self.parse_set()?;
                self.next_must(Token::Keyword(Keyword::In))?;
                let base = self.parse_base()?;
                Ok(Fragment::For(variables, base))
            }
            Keyword::Let => {
                let left = self.parse_identifier()?;
                self.next_must(Token::Assign)?;
                let right = self.parse_base()?;
                Ok(Fragment::Let(left, right))
            }
            Keyword::Include => {
                let name = self.parse_base()?;
                let scope = self.parse_mount()?;
                Ok(Fragment::Include(name, scope))
            }
            Keyword::Extends => {
                let name = self.parse_base()?;
                Ok(Fragment::Extends(name))
            }
            Keyword::Block => {
                let name = self.parse_base()?;
                Ok(Fragment::Block(name))
            }
            Keyword::End => Ok(Fragment::End),
            k @ Keyword::Not | k @ Keyword::In => Err(Error::build(UNEXPECTED_TOKEN)
                .with_pointer(self.lexer.source, region)
                .with_help(format!("keyword `{k}` is not valid in this position"))),
        }
    }

    /// Parse a [`Mount`].
    ///
    /// Similar to `.parse_arguments`, but requires that all arguments are named.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, any of the [`Argument`] instances
    /// are not named, or no `Token` instances are left.
    fn parse_mount(&mut self) -> Result<Option<Mount>, Error> {
        // "(* include "header" title: site.title, date: "01-01-22" *)"
        //                      -----------------| ---------------- ...
        //                          argument 1   |    argument 2
        //                     ^-----------------|-----------------^
        //                     from              |                 to
        //                                    separator
        let mut values: Vec<Point> = vec![];

        while !self.peek_is(Token::EndBlock)? {
            let argument = self.parse_argument()?;

            if argument.name.is_none() {
                return Err(Error::build(INVALID_SYNTAX)
                    .with_pointer(self.lexer.source, argument.value.get_region())
                    .with_help(format!(
                        "arguments in `include` expression must be named, try `[name: ]{}`",
                        argument.value.get_region().literal(self.lexer.source)
                    )));
            }
            values.push(Point {
                name: argument.name.unwrap(),
                value: argument.value,
            });

            if !self.peek_is(Token::Comma)? {
                break;
            }
            self.next_must(Token::Comma)?;
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
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
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

    /// Parse an [`IfTree`].
    ///
    /// This `IfTree` will contain all of the information necessary to determine if the
    /// block should pass.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
    fn parse_tree(&mut self) -> Result<IfTree, Error> {
        // this >= that && these == those || a == b *)
        // |-----------    --------------    ------ to
        // from  |                  |            |
        //       negatable          negatable    negatable
        let mut tree = IfTree::new();
        let mut state = IfState::default();

        loop {
            match state {
                IfState::Base(has_left) => {
                    if has_left {
                        let base = self.parse_base()?;
                        tree.last_leaf_mut_must("has_left implies that a leaf exists")
                            .right = Some(base);

                        state = IfState::Transition;
                    } else {
                        let (base, negated) = if self.peek_is(Token::Keyword(Keyword::Not))? {
                            self.next_must(Token::Keyword(Keyword::Not))?;

                            (self.parse_base()?, true)
                        } else {
                            (self.parse_base()?, false)
                        };
                        tree.last_mut_must().push(IfLeaf::new(base, negated));

                        state = IfState::Operator;
                    };
                }
                IfState::Operator => match self.peek_must()? {
                    (token, region) => match token {
                        Token::EndBlock | Token::Or | Token::And => state = IfState::Transition,
                        Token::Operator(op) => {
                            self.next_must(Token::Operator(op))?;
                            tree.last_leaf_mut_must("operator state implies that leaf exists")
                                .operator = Some(op);
                            state = IfState::Base(true);
                        }
                        unexpected => {
                            return Err(Error::build(UNEXPECTED_TOKEN)
                                .with_pointer(self.lexer.source, region)
                                .with_help(expected_operator(unexpected)))
                        }
                    },
                },
                IfState::Transition => match self.peek_must()? {
                    (Token::EndBlock, _) => break,
                    (Token::Or, _) => {
                        self.next_must(Token::Or)?;

                        tree.split_branch();
                        state = IfState::Base(false);
                    }
                    (Token::And, _) => {
                        self.next_must(Token::And)?;

                        let (base, negated) = if self.peek_is(Token::Keyword(Keyword::Not))? {
                            self.next_must(Token::Keyword(Keyword::Not))?;
                            (self.parse_base()?, true)
                        } else {
                            (self.parse_base()?, false)
                        };

                        tree.split_leaf(IfLeaf::new(base, negated));
                        state = IfState::Operator;
                    }
                    unexpected => {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .with_pointer(self.lexer.source, unexpected.1)
                            .with_help(format!(
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
                .with_help(format!(
                    "expected keyword like `if`, `else`, `let`, `for`, `in`, `include`, \
                    `extends`, `block`, `end`, found `{token}`"
                ))
                .with_pointer(self.lexer.source, region)),
        }
    }

    /// Parse an [`Arguments`].
    ///
    /// If no arguments exist, [`None`] is returned instead.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
    fn parse_arguments(&mut self) -> Result<Option<Arguments>, Error> {
        // "hello (( name | prepend 1: "hello, " | append "!", "?" | upper ))"
        //                         ^------------^        ^--------^
        //                         from         to       from     to
        let mut values: Vec<Argument> = vec![];

        while !self.peek_is(Token::Pipe)? && !self.peek_is(Token::EndExpression)? {
            values.push(self.parse_argument()?);
            if !self.peek_is(Token::Comma)? {
                break;
            }
            self.next_must(Token::Comma)?;
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
    /// Returns an [`Error`] if a valid `Argument` cannot be made with the next tokens.
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
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
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
                    if !region.is_neighbor(next_region) {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .with_pointer(self.lexer.source, region.combine(next_region))
                            .with_help(format!(
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
                        .with_pointer(self.lexer.source, region)
                        .with_help(format!(
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

                while self.peek_is(Token::Period)? {
                    self.next_must(Token::Period)?;
                    path.push(self.parse_key()?);
                }
                Base::Variable(Variable::new(path))
            }
            (token, region) => {
                println!("{}", token);
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .with_pointer(self.lexer.source, region)
                    .with_help(format!(
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
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn parse_string_literal(&mut self, region: Region) -> Result<Literal, Error> {
        let value = Value::String(self.parse_string(region)?);

        Ok(Literal { value, region })
    }

    /// Parse a [`Key`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the next token is not a valid [`Identifier`].
    fn parse_key(&mut self) -> Result<Identifier, Error> {
        match self.next_any_must()? {
            (Token::Identifier, region) => Ok(Identifier { region }),
            (_, region) => Err(Error::build(UNEXPECTED_TOKEN)
                .with_pointer(self.lexer.source, region)
                .with_help("expected an unquoted identifier such as `one.two`")),
        }
    }

    /// Parse a [`String`] from the literal value of the given [`Region`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if an unrecognized escape character is found.
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
                                    .with_pointer(self.lexer.source, region))
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
    /// Returns an [`Error`] if the literal value of the `Region` cannot be converted
    /// to a [`Value::Number`].
    fn parse_number_literal(&self, window: &str, region: Region) -> Result<Literal, Error> {
        let as_number: Number = window.parse().map_err(|_| {
            Error::build("unrecognizable number")
                .with_pointer(self.lexer.source, region)
                .with_help(format!(
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
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn peek(&mut self) -> TokenResult {
        if let o @ None = &mut self.buffer {
            *o = Some(self.lexer.next()?);
        }

        Ok(self.buffer.unwrap())
    }

    /// Peek the next [`Token`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the [`Lexer`] returns [`None`], or when the `Lexer` returns
    /// an error itself.
    fn peek_must(&mut self) -> TokenResultMust {
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
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
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
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn next(&mut self) -> TokenResult {
        match self.buffer.take() {
            Some(t) => Ok(t),
            None => self.lexer.next(),
        }
    }

    /// Get the next [`Token`], and compare it to the given `Token`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
    fn next_must(&mut self, expect: Token) -> TokenResultMust {
        match self.next()? {
            Some((token, region)) => {
                if token == expect {
                    Ok((token, region))
                } else {
                    Err(Error::build(UNEXPECTED_TOKEN)
                        .with_pointer(self.lexer.source, region)
                        .with_help(format!("expected `{expect}`")))
                }
            }
            None => {
                let source_len = self.lexer.source.len();
                Err(Error::build(UNEXPECTED_EOF)
                    .with_pointer(self.lexer.source, source_len..source_len)
                    .with_help(format!("expected `{expect}`")))
            }
        }
    }

    /// Get the next [`Token`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found, or no `Token`
    /// instances are left.
    fn next_any_must(&mut self) -> TokenResultMust {
        match self.next()? {
            Some((token, region)) => Ok((token, region)),
            None => Err(error_eof(self.lexer.source)),
        }
    }
}

/// Return an [`Error`] describing an unexpected end of file.
fn error_eof(source: &str) -> Error {
    let source_len = source.len();
    Error::build(UNEXPECTED_EOF)
        .with_pointer(source, source_len..source_len)
        .with_help("expected additional tokens, did you close all blocks and expressions?")
}

#[cfg(test)]
mod tests {
    use morel::Finder;

    use crate::{
        compile::{lex::token::Token, tree::Set},
        region::Region,
        Builder,
    };

    use super::{
        tree::{Expression, Tree},
        Parser,
    };

    #[test]
    fn test_parser_lexer_integration() {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut parser = Parser::new("hello", &finder);

        assert_eq!(parser.next(), Ok(Some((Token::Raw, (0..5).into()))));
        assert_eq!(parser.next(), Ok(None));
    }

    #[test]
    fn test_parse_expression_with_call() {
        let source = r#"hello (( name | prepend text: "hello, " | append "!", "?" | upper ))"#;
        let template = Parser::new(source, &Finder::new(Builder::new().to_syntax()))
            .compile(None)
            .unwrap();

        let mut iterator = template.get_scope().data.iter();
        iterator.next();

        match iterator.next().unwrap() {
            Tree::Output(output) => match &output.expression {
                Expression::Call(call) => {
                    assert_eq!(call.name.region.literal(source), "upper");

                    match call.receiver.as_ref() {
                        Expression::Call(call) => {
                            let arguments = call.arguments.as_ref().unwrap();
                            let first = arguments.values.first().unwrap();
                            let second = arguments.values.last().unwrap();

                            assert_eq!(call.name.region.literal(source), "append");
                            assert_eq!(
                                call.region.literal(source),
                                r#"name | prepend text: "hello, " | append "!", "?""#
                            );
                            assert_eq!(first.get_region().literal(source), r#""!""#);
                            assert_eq!(second.get_region().literal(source), r#""?""#);
                            assert_eq!(arguments.region.literal(source), r#""!", "?""#)
                        }
                        _ => panic!("filter `upper` does not lead to call type expression"),
                    }
                }
                _ => panic!("expected `upper` to be call type expression"),
            },
            _ => panic!("expected `output` type expression"),
        }
    }

    #[test]
    fn test_parse_negative_num_err() {
        let source = "balance: (( - 1000 ))";
        //                         ^-- remove whitespace for negative num

        assert!(
            Parser::new(source, &Finder::new(Builder::new().to_syntax()))
                .compile(None)
                .is_err()
        );
    }

    #[test]
    fn test_peek_multiple() {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut parser = Parser::new("(( one two", &finder);

        assert!(parser.next().is_ok());

        // Multiple calls to peek should return the same thing.
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
        assert_eq!(parser.peek(), Ok(Some((Token::Identifier, (3..6).into()))));
    }

    #[test]
    fn test_parse_tree_valid() {
        let finder = Finder::new(Builder::new().to_syntax());
        //                                                    --- negated
        let source = "(* if this >= that && these == those || not is_admin *)";
        //                  ------------    --------------    ------------
        //                     Leaf 1           Leaf 2           Leaf 1
        //                  ------------------------------    ------------
        //                              Branch 1                 Branch 2
        //                  ----------------------------------------------
        //                                         Tree
        let result = get_parser_n(source, &finder, 2).parse_tree().unwrap();

        assert_eq!(result.branches.len(), 2);
        assert_eq!(result.branches.first().unwrap().len(), 2);
        assert_eq!(result.branches.last().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_tree_missing_base() {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut parser = get_parser_n("(* if this >= *)", &finder, 2);
        //                                          ^-- expected `Base` here

        assert!(parser.parse_tree().is_err());
    }

    #[test]
    fn test_parse_tree_bad_operator() {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut parser = get_parser_n("(* if this = that *)", &finder, 2);
        //                                        ^-- did you mean `==`?

        assert!(parser.parse_tree().is_err());
    }

    #[test]
    fn test_parse_set_pair() {
        let finder = Finder::new(Builder::new().to_syntax());
        //                         ---- identifier 2
        let source = "(* for this, that in thing *)hello(* end *)";
        //      identifier 1 ----     base -----   ----- scope
        let result = get_parser_n(source, &finder, 2).parse_set().unwrap();

        match result {
            Set::Pair(pa) => {
                assert_eq!(pa.key.region.literal(source), "this");
                assert_eq!(pa.value.region.literal(source), "that");
                assert_eq!(pa.region.literal(source), "this, that");
            }
            _ => panic!("two identifiers must create pair"),
        }
    }

    #[test]
    fn test_parse_mount() {
        let finder = Finder::new(Builder::new().to_syntax());
        //                                        -------- name
        let mut parser = get_parser_n("(* include \"base\" x: abc, y: def *)", &finder, 3);
        //                                                 -------------- mount
        let result = parser.parse_mount();
        let mount = result.unwrap().unwrap();

        assert_eq!(mount.values.len(), 2);
        assert_eq!(
            parser.next(),
            Ok(Some((Token::EndBlock, Region { begin: 33, end: 35 })))
        );
    }

    #[test]
    fn test_parse_block() {
        //                     ---- name
        let source = "(* block main *)abc(* end *)def";
        let template = get_parser_n(source, &Finder::new(Builder::new().to_syntax()), 0)
            .compile(None)
            .unwrap();

        match template.get_scope().data.first().unwrap() {
            Tree::Block(block) => {
                assert_eq!(block.name.get_region().literal(source), "main");
                match block.scope.data.first().unwrap() {
                    Tree::Raw(raw) => assert_eq!(raw.literal(source), "abc"),
                    _ => panic!("unexpected block scope"),
                }
            }
            _ => panic!("expected block"),
        }
        match template.get_scope().data.last().unwrap() {
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
    fn get_parser_n<'source>(
        source: &'source str,
        finder: &'source Finder,
        n: i8,
    ) -> Parser<'source> {
        let mut parser = Parser::new(source, finder);
        for _ in 0..n {
            parser.next().unwrap();
        }

        parser
    }
}
