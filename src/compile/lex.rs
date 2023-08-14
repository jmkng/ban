pub mod token;

mod state;

use crate::{log::Error, region::Region};

use super::{
    expected_operator, lex::state::CursorState, token::Token, Keyword, Operator, TokenResult,
    INVALID_SYNTAX, UNEXPECTED_TOKEN,
};

use morel::Finder;

/// Provides methods to read a source string as [`Token`] instances.
pub struct Lexer<'source> {
    /// Reference to the source text.
    pub source: &'source str,
    /// Position within source.
    pub cursor: usize,
    /// Compiled [`Finder`] instance used to search for markers
    /// in the source text.
    finder: &'source Finder,
    /// Tracks the [`Lexer`] state and determines the action taken
    /// when `.next` is called.
    state: CursorState,
    /// When true, the following [`Token`] read while in
    /// [`CursorState::Default`] state will be left trimmed.
    left_trim: bool,
    /// Temporary storage for the a [`Token`] that will be read
    /// on the following call to `.next`
    buffer: Option<(Token, Region)>,
}

impl<'source> Lexer<'source> {
    /// Create a new [`Lexer`] from the given source and [`Syntax`].
    #[inline]
    pub fn new(source: &'source str, finder: &'source Finder) -> Self {
        Self {
            finder,
            state: CursorState::Default,
            source,
            left_trim: false,
            cursor: 0,
            buffer: None,
        }
    }

    /// Return the next [`Token`] and [`Region`].
    ///
    /// Any instance of [`Token::Whitespace`] is ignored.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    pub fn next(&mut self) -> TokenResult {
        loop {
            // Always prefer taking from the buffer when possible.
            if let Some(next) = self.buffer.take() {
                return Ok(Some(next));
            }
            if self.source[self.cursor..].is_empty() {
                return Ok(None);
            }

            let c = self.cursor;
            let result = match self.state {
                CursorState::Default => self.lex_default(c),
                CursorState::Inside { .. } => self.lex_tag(c),
            }?;

            return match result {
                Some((token, region)) => match token {
                    Token::Whitespace => continue,
                    _ => Ok(Some((token, region))),
                },
                None => Ok(None),
            };
        }
    }

    /// Return the next [`Token`] and [`Region`] in [`Tag`][`CursorState::Inside`]
    /// configuration.
    ///
    /// Assumes the cursor is inside of an expression.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn lex_tag(&mut self, from: usize) -> TokenResult {
        match self.finder.starts(self.source, from) {
            Some((id, length)) => {
                let (token, is_trimmed) = Token::from_usize_trim(id);

                match self.state {
                    CursorState::Inside { ref end_token } => {
                        if token == *end_token {
                            self.state = CursorState::Default;
                            self.left_trim = is_trimmed;
                            self.cursor = length;

                            Ok(Some((token, (from..length).into())))
                        } else {
                            let which = if *end_token == Token::EndExpression {
                                "expression"
                            } else {
                                "block"
                            };

                            Err(Error::build(UNEXPECTED_TOKEN)
                                .with_pointer(self.source, from..length)
                                .with_help(format!("did you close the previous {which}?")))
                        }
                    }
                    _ => panic!("lexer must be in tag state"),
                }
            }
            None => {
                let mut advance = |length: usize, data: Token| {
                    self.cursor += length;

                    Ok(Some((data, (from..from + length).into())))
                };

                let mut iterator = self.source[from..]
                    .char_indices()
                    .map(|(d, c)| (from + d, c));
                let (index, char) = iterator.next().unwrap();

                match char {
                    '*' => advance(1, Token::Operator(Operator::Multiply)),
                    '+' => advance(1, Token::Operator(Operator::Add)),
                    '/' => advance(1, Token::Operator(Operator::Divide)),
                    '-' => advance(1, Token::Operator(Operator::Subtract)),
                    '.' => advance(1, Token::Period),
                    ',' => advance(1, Token::Comma),
                    ':' => advance(1, Token::Colon),
                    '"' => self.lex_string(iterator, index),
                    '=' | '!' | '>' | '<' | '|' | '&' => self.lex_operator(iterator, index, char),
                    c if c.is_whitespace() => Ok(Some(self.lex_whitespace(iterator, index))),
                    c if c.is_ascii_digit() => Ok(Some(self.lex_digit(iterator, index))),
                    c if is_ident_start(c) => Ok(Some(self.lex_ident_or_keyword(iterator, index))),
                    _ => Err(Error::build(UNEXPECTED_TOKEN)
                        .with_pointer(self.source, index..index + char.len_utf8())
                        .with_help(
                            "expected one of `*`, `+`, `/`, `-`, `.`, `:`, an identifier, \
                            an ascii digit, or beginning of a string literal marked with `\"`",
                        )),
                }
            }
        }
    }

    /// Return a [`Token`] and [`Region`] based on the previous character.
    ///
    /// Checks the next character via `.next` to ensure the correct `Token` is
    /// returned. All of these are recognized:
    ///
    /// `==`, `!=`, `>=`, `<=`, `||`, `&&`, `=`, `|`, `!`
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn lex_operator<T>(&mut self, mut iter: T, from: usize, previous: char) -> TokenResult
    where
        T: Iterator<Item = (usize, char)>,
    {
        let (position, token) = match (previous, iter.next()) {
            // Double:
            ('=', Some((usize, '='))) => (usize, Token::Operator(Operator::Equal)),
            ('!', Some((usize, '='))) => (usize, Token::Operator(Operator::NotEqual)),
            ('>', Some((usize, '='))) => (usize, Token::Operator(Operator::GreaterOrEqual)),
            ('<', Some((usize, '='))) => (usize, Token::Operator(Operator::LesserOrEqual)),
            ('|', Some((usize, '|'))) => (usize, Token::Or),
            ('&', Some((usize, '&'))) => (usize, Token::And),
            // Single:
            ('=', _) => (from, Token::Assign),
            ('|', _) => (from, Token::Pipe),
            ('!', _) => (from, Token::Exclamation),
            ('>', _) => (from, Token::Operator(Operator::Greater)),
            ('<', _) => (from, Token::Operator(Operator::Lesser)),
            _ => {
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .with_pointer(self.source, from..from + 1)
                    .with_help(expected_operator(previous)));
            }
        };
        let position = position + 1;
        self.cursor = position;

        Ok(Some((token, (from..position).into())))
    }

    /// Return a [`Token`] and [`Region`] containing [`Token::Number`].
    fn lex_digit<T>(&mut self, mut iter: T, from: usize) -> (Token, Region)
    where
        T: Iterator<Item = (usize, char)>,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !is_number(char) => {
                    self.cursor = index;

                    break (Token::Number, (from..index).into());
                }
                Some((_, _)) => continue,
                None => return (Token::Number, (from..self.source.len()).into()),
            }
        }
    }

    /// Return a [`Token`] and [`Region`] containing [`Token::Whitespace`].
    fn lex_whitespace<T>(&mut self, mut iter: T, from: usize) -> (Token, Region)
    where
        T: Iterator<Item = (usize, char)>,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !char.is_whitespace() => {
                    self.cursor = index;

                    break (Token::Whitespace, (from..index).into());
                }
                Some((_, _)) => continue,
                None => return (Token::Whitespace, (from..self.source.len()).into()),
            }
        }
    }

    /// Return a [`Token`] and [`Region`] containing [`Token::String`] using
    /// the given iterator.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn lex_string<T>(&mut self, mut iter: T, from: usize) -> TokenResult
    where
        T: Iterator<Item = (usize, char)>,
    {
        let mut previous = (from, '"');
        loop {
            match iter.next() {
                Some((index, '"')) if previous.1 != '\\' => {
                    // Accept a double quote as a signal to end the string, unless the previous
                    // character was an escape.
                    //
                    // Add one to the index of the character to comply with string slice
                    // semantics.
                    let to = index + 1;
                    self.cursor = to;

                    return Ok(Some((Token::String, (from..to).into())));
                }
                Some((index, char)) => {
                    // Assign character to "previous" and move on. We use "previous" to
                    // determine if a double quote should be escaped.
                    previous = (index, char);
                }
                None => {
                    let take = if previous.0 - from > 10 {
                        10
                    } else {
                        previous.0
                    };

                    return Err(Error::build(INVALID_SYNTAX)
                        .with_pointer(self.source, from..take)
                        .with_help(
                            "this might be an undelimited string, try closing it with `\"`",
                        ));
                }
            }
        }
    }

    /// Return a [`Token`] and [`Region`] from the given iterator.
    ///
    /// The `Token` will be [`Token::Identifier`] or [`Token::Keyword`].
    fn lex_ident_or_keyword<T>(&mut self, mut iter: T, from: usize) -> (Token, Region)
    where
        T: Iterator<Item = (usize, char)>,
    {
        let mut check_keyword = |to: usize| {
            let range_text = self
                .source
                .get(from..to)
                .expect("valid range is required to check keyword");

            let token = match range_text {
                "not" => Token::Keyword(Keyword::Not),
                "if" => Token::Keyword(Keyword::If),
                "else" => Token::Keyword(Keyword::Else),
                "let" => Token::Keyword(Keyword::Let),
                "for" => Token::Keyword(Keyword::For),
                "in" => Token::Keyword(Keyword::In),
                "include" => Token::Keyword(Keyword::Include),
                "extends" => Token::Keyword(Keyword::Extends),
                "block" => Token::Keyword(Keyword::Block),
                "end" => Token::Keyword(Keyword::End),
                "true" => Token::True,
                "false" => Token::False,
                _ => Token::Identifier,
            };
            self.cursor = to;

            (token, (from..to).into())
        };

        loop {
            match iter.next() {
                Some((index, char)) if !is_ident_continue(char) => {
                    break check_keyword(index);
                }
                Some((_, _)) => continue,
                None => break check_keyword(self.source.len()),
            }
        }
    }

    /// Return the next [`Token`] and [`Region`] in [`Tag`][`CursorState::Default`]
    /// configuration.
    ///
    /// Assumes the cursor is outside of an expression.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when an unexpected [`Token`] is found.
    fn lex_default(&mut self, from: usize) -> TokenResult {
        let mut trim_region = |mut region_begin, mut region_end, right_trim| {
            if right_trim {
                region_end = self.source[..region_end].trim_end().len();
            }
            if self.left_trim {
                self.left_trim = false;
                let s = &self.source[region_begin..region_end];
                region_begin = region_begin + s.len() - s.trim_start().len()
            }

            Ok(Some((Token::Raw, (region_begin..region_end).into())))
        };

        match self.finder.next(self.source, from) {
            Some((id, marker_begin, marker_end)) => {
                let (token, is_trimmed) = Token::from_usize_trim(id);

                match &token {
                    Token::BeginExpression => {
                        self.state = CursorState::Inside {
                            end_token: Token::EndExpression,
                        }
                    }
                    Token::BeginBlock => {
                        self.state = CursorState::Inside {
                            end_token: Token::EndBlock,
                        }
                    }
                    _ => {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .with_pointer(self.source, marker_begin..marker_end)
                            .with_help("expected beginning expression or beginning block"));
                    }
                }

                if from == marker_begin {
                    self.cursor = marker_end;

                    Ok(Some((token, (marker_begin..marker_end).into())))
                } else {
                    self.cursor = marker_end;
                    self.buffer = Some((token, (marker_begin..marker_end).into()));

                    trim_region(from, marker_begin, is_trimmed)
                }
            }
            None => {
                let remaining = self.cursor..self.source.len();
                self.cursor = self.source.len();

                trim_region(from, remaining.end, false)
            }
        }
    }
}

/// Return true if the given character is a recognized beginning identifier,
/// meaning '_' or an `xid_start`.
fn is_ident_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

/// Return true if the given character is a recognized continue identifier,
/// meaning an `xid_continue`.
fn is_ident_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

/// Return true if the given character is a number (0-9) or a period.
fn is_number(c: char) -> bool {
    matches!(c, '0'..='9' | '.')
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Write,
        fs::File,
        io::{BufRead, BufReader},
    };

    use crate::{
        compile::{
            lex::{state::CursorState, Token},
            Keyword, Operator,
        },
        log::Error,
        region::Region,
        Builder,
    };

    use super::Lexer;

    use morel::Finder;

    #[test]
    fn test_lex_default_no_match() {
        let expect = vec![(Token::Raw, 0..11)];

        helper_lex_next_auto("lorem ipsum", expect)
    }

    #[test]
    fn test_lex_default_match_no_trim() {
        let expect = vec![
            (Token::Raw, 0..12),
            (Token::BeginExpression, 12..14),
            (Token::Identifier, 15..20),
        ];

        helper_lex_next_auto("lorem ipsum (( dolor", expect);
    }

    #[test]
    fn test_lex_default_match_trim() {
        let expect = vec![
            (Token::Raw, 0..11),
            (Token::BeginExpression, 12..15),
            (Token::Identifier, 16..21),
        ];

        helper_lex_next_auto("lorem ipsum ((- dolor", expect);
    }

    #[test]
    fn test_lex_state_change() -> Result<(), Error> {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut block_lexer = Lexer::new("lorem (*", &finder);
        let mut expression_lexer = Lexer::new("lorem ((", &finder);
        block_lexer.next()?;
        expression_lexer.next()?;

        assert_eq!(
            block_lexer.state,
            CursorState::Inside {
                end_token: Token::EndBlock
            }
        );
        assert_eq!(
            expression_lexer.state,
            CursorState::Inside {
                end_token: Token::EndExpression
            }
        );

        Ok(())
    }

    #[test]
    fn test_lex_digit() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::Number, 3..5),
            (Token::EndExpression, 6..8),
        ];

        helper_lex_next_auto("(( 10 ))", expect);
    }

    #[test]
    fn test_lex_ident() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::Identifier, 3..8),
            (Token::EndExpression, 9..11),
        ];

        helper_lex_next_auto("(( hello ))", expect);
    }

    #[test]
    fn test_lex_keyword() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::Keyword(Keyword::If), 3..5),
            (Token::EndExpression, 6..8),
        ];

        helper_lex_next_auto("(( if ))", expect);
    }

    #[test]
    fn test_lex_string_escape() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::String, 3..13),
            (Token::EndExpression, 14..16),
        ];

        helper_lex_next_auto(r#"(( "\"name\"" ))"#, expect);
    }

    #[test]
    fn test_lex_full_document() {
        // Reading the file this way should strip any \r\n line endings and replace
        // them with \n, so the test should pass on Windows.
        let mut source = String::new();
        let file = File::open("examples/template.html").unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            write!(source, "{}\n", line.unwrap()).unwrap();
        }
        source = source.strip_suffix('\n').unwrap().to_string();
        let expect = vec![
            (Token::Raw, 0..280),
            (Token::BeginExpression, 280..282),
            (Token::Identifier, 283..287),
            (Token::EndExpression, 288..290),
            (Token::Raw, 290..309),
            (Token::BeginBlock, 309..311),
            (Token::Keyword(Keyword::For), 312..315),
            (Token::Identifier, 316..322),
            (Token::Keyword(Keyword::In), 323..325),
            (Token::Identifier, 326..332),
            (Token::EndBlock, 333..335),
            (Token::Raw, 335..352),
            (Token::BeginExpression, 352..354),
            (Token::Identifier, 355..361),
            (Token::Period, 361..362),
            (Token::Identifier, 362..366),
            (Token::EndExpression, 367..369),
            (Token::Raw, 369..383),
            (Token::BeginBlock, 383..385),
            (Token::Keyword(Keyword::End), 386..389),
            (Token::EndBlock, 390..392),
            (Token::Raw, 392..408),
            (Token::BeginBlock, 408..410),
            (Token::Keyword(Keyword::Let), 411..414),
            (Token::Identifier, 415..418),
            (Token::Assign, 419..420),
            (Token::String, 421..426),
            (Token::EndBlock, 427..429),
            (Token::Raw, 429..434),
            (Token::BeginBlock, 434..436),
            (Token::Keyword(Keyword::Let), 437..440),
            (Token::Identifier, 441..444),
            (Token::Assign, 445..446),
            (Token::String, 447..452),
            (Token::EndBlock, 453..455),
            (Token::Raw, 455..461),
            (Token::BeginBlock, 461..463),
            (Token::Keyword(Keyword::Include), 464..471),
            (Token::Identifier, 472..478),
            (Token::Identifier, 479..480),
            (Token::Colon, 480..481),
            (Token::Identifier, 482..485),
            (Token::Comma, 485..486),
            (Token::Identifier, 487..488),
            (Token::Colon, 488..489),
            (Token::Identifier, 490..493),
            (Token::EndBlock, 494..496),
            (Token::Raw, 496..502),
            (Token::BeginBlock, 502..504),
            (Token::Keyword(Keyword::If), 505..507),
            (Token::Identifier, 508..512),
            (Token::Operator(Operator::Equal), 513..515),
            (Token::String, 516..524),
            (Token::EndBlock, 525..527),
            (Token::Raw, 527..569),
            (Token::BeginBlock, 569..571),
            (Token::Keyword(Keyword::End), 572..575),
            (Token::EndBlock, 576..578),
            (Token::Raw, 578..588),
            (Token::BeginBlock, 588..590),
            (Token::Keyword(Keyword::Block), 591..596),
            (Token::Identifier, 597..601),
            (Token::EndBlock, 602..604),
            (Token::Raw, 604..610),
            (Token::BeginBlock, 610..612),
            (Token::Keyword(Keyword::End), 613..616),
            (Token::EndBlock, 617..619),
            (Token::Raw, 619..630),
            (Token::BeginExpression, 630..632),
            (Token::Identifier, 633..637),
            (Token::Pipe, 638..639),
            (Token::Identifier, 640..647),
            (Token::Number, 648..649),
            (Token::Colon, 649..650),
            (Token::String, 651..660),
            (Token::Pipe, 661..662),
            (Token::Identifier, 663..669),
            (Token::String, 670..673),
            (Token::Comma, 673..674),
            (Token::String, 675..678),
            (Token::Pipe, 679..680),
            (Token::Identifier, 681..693),
            (Token::EndExpression, 694..696),
            (Token::Raw, 696..712),
        ];

        helper_lex_next_auto(&source, expect)
    }

    #[test]
    fn test_lex_string() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::String, 3..9),
            (Token::EndExpression, 10..12),
        ];

        helper_lex_next_auto("(( \"name\" ))", expect);
    }

    #[test]
    fn test_error_multiple_opening_tags() {
        let expect = vec![
            (Token::Raw, 0..6),
            (Token::BeginExpression, 6..8),
            (Token::Identifier, 9..13),
        ];

        let finder = Finder::new(Builder::new().to_syntax());
        let mut lexer = Lexer::new("hello (( name (( ))", &finder);
        for (token, range) in expect {
            assert_eq!(lexer.next(), Ok(Some((token, range.into()))))
        }

        assert!(lexer.next().is_err())
    }

    /// Helper function which takes in a source string, creates a lexer on that
    /// string and iterates [expect.len()] amount of times and compares the result
    /// against [lexer.next()].
    fn helper_lex_next_auto<T>(source: &str, expect: Vec<(Token, T)>)
    where
        T: Into<Region>,
    {
        let finder = Finder::new(Builder::new().to_syntax());
        let mut lexer = Lexer::new(source, &finder);
        for (token, region) in expect {
            assert_eq!(lexer.next(), Ok(Some((token, region.into()))))
        }

        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
    }
}
