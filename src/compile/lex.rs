pub mod token;

mod state;

use crate::{
    compile::{lex::state::State, token::Token, Keyword, Operator},
    log::{expected_operator, Error, INVALID_SYNTAX, UNEXPECTED_TOKEN},
    region::Region,
    Builder,
};
use morel::Finder;

pub type LexResult = Result<Option<(Token, Region)>, Error>;
pub type LexResultMust = Result<(Token, Region), Error>;

/// Provides methods to iterate over a source string and receive Token instances
/// instead of characters or bytes.
pub struct Lexer<'source> {
    /// Text that is being analyzed.
    pub source: &'source str,
    /// Position within source.
    pub cursor: usize,
    /// Utility for searching text for patterns.
    finder: Finder<&'source str>,
    /// State determines the action taken when [.next()] is called.
    state: State,
    /// When true, the following Token pulled in State::Default state
    /// will be left trimmed.
    left_trim: bool,
    /// Temporary storage for a "next" token that will be read
    /// on the following call to [.next()]
    buffer: Option<(Token, Region)>,
}

impl<'source> Lexer<'source> {
    /// Create a new Lexer from the given string.
    #[inline]
    pub fn new(source: &'source str) -> Self {
        Self {
            finder: Finder::new(Builder::new().build()),
            state: State::Default,
            source,
            left_trim: false,
            cursor: 0,
            buffer: None,
        }
    }

    /// Lex the next token.
    ///
    /// Any instance of Token::Whitespace is ignored.
    ///
    /// # Errors
    ///
    /// Propagates an errors reported by individual Lexer methods such as lex_default,
    /// which occur when incorrect syntax is detected.
    pub fn next(&mut self) -> LexResult {
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
                State::Default => self.lex_default(c),
                State::Tag { .. } => self.lex_tag(c),
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

    /// Lex the next token.
    ///
    /// Behaves as if the cursor is inside of a tag, which can be either a block or expression.
    fn lex_tag(&mut self, from: usize) -> LexResult {
        match self.finder.starts(self.source, from) {
            Some((id, length)) => {
                let (token, is_trimmed) = Token::from_usize_trim(id);

                match self.state {
                    State::Tag { ref end_token } => {
                        // Matching token.
                        if token == *end_token {
                            self.state = State::Default;
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
                                .pointer(self.source, from..length)
                                .help(format!("did you close the previous {which}?")))
                        }
                    }
                    _ => panic!("unexpected state for lex_tag"),
                }
            }
            None => {
                // An iterator over the remaining source, with the index adjusted to match the
                // true position based on our "from" parameter.
                let mut iterator = self.source[from..]
                    .char_indices()
                    .map(|(d, c)| (from + d, c));

                let mut advance = |length: usize, data: Token| {
                    self.cursor += length;
                    Ok(Some((data, (from..from + length).into())))
                };

                let (index, char) = iterator.next().unwrap();
                match char {
                    '*' => advance(1, Token::Operator(Operator::Multiply)),
                    '+' => advance(1, Token::Operator(Operator::Add)),
                    '/' => advance(1, Token::Operator(Operator::Divide)),
                    '-' => advance(1, Token::Operator(Operator::Subtract)),
                    '.' => advance(1, Token::Period),
                    ':' => advance(1, Token::Colon),
                    c if c.is_whitespace() => Ok(Some(self.lex_whitespace(iterator, index))),
                    c if c.is_ascii_digit() => Ok(Some(self.lex_digit(iterator, index))),
                    c if is_ident_start(c) => Ok(Some(self.lex_ident_or_keyword(iterator, index))),
                    '"' => self.lex_string(iterator, index),
                    // Below characters could mean one thing or another depending on the character
                    // after it, lex_operator will figure it out and return the right thing.
                    '=' | '!' | '>' | '<' | '|' | '&' => self.lex_operator(iterator, index, char),
                    _ => Err(Error::build(UNEXPECTED_TOKEN)
                        .pointer(self.source, index..index + 1)
                        .help(
                            "expected one of `*`, `+`, `/`, `-`, `.`, `:`, an identifier, \
                            an ascii digit, or beginning of a string literal marked with \"",
                        )),
                }
            }
        }
    }

    /// Lex a Region that may contain a Token::Operator,
    ///
    /// Receives an iterator (iter) over the remaining text, and the index (from) and
    /// char (previous) of the previously read character.
    ///
    /// The information about the previous character is required to determine if it
    /// and the next character form a larger operator such as `==`.
    ///
    /// # Examples
    ///
    /// This function will recognize the following combinations: `==`, `!=`,
    /// `>=`, `<=`, `||`, `&&`, `=`, `|`, `!`
    ///
    /// # Errors
    ///
    /// Returns an error if one of the patterns described above is not matched.
    fn lex_operator<T>(&mut self, mut iter: T, from: usize, previous: char) -> LexResult
    where
        T: Iterator<Item = (usize, char)>,
    {
        let (position, token) = match (previous, iter.next()) {
            // Double operators:
            ('=', Some((usize, '='))) => (usize, Token::Operator(Operator::Equal)),
            ('!', Some((usize, '='))) => (usize, Token::Operator(Operator::NotEqual)),
            ('>', Some((usize, '='))) => (usize, Token::Operator(Operator::GreaterOrEqual)),
            ('<', Some((usize, '='))) => (usize, Token::Operator(Operator::LesserOrEqual)),
            ('|', Some((usize, '|'))) => (usize, Token::Or),
            ('&', Some((usize, '&'))) => (usize, Token::And),
            // Single operators:
            ('=', _) => (from, Token::Assign),
            ('|', _) => (from, Token::Pipe),
            ('!', _) => (from, Token::Exclamation),
            ('>', _) => (from, Token::Operator(Operator::Greater)),
            ('<', _) => (from, Token::Operator(Operator::Lesser)),
            _ => {
                return Err(Error::build(UNEXPECTED_TOKEN)
                    .pointer(self.source, from..from + 1)
                    .help(expected_operator(previous)));
            }
        };
        let position = position + 1;

        self.cursor = position;
        Ok(Some((token, (from..position).into())))
    }

    /// Lex a Region containing a Token::Number.
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

    /// Lex a Region containing a Token::Whitespace.
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

    /// Lex a (Token::String, Region) with the given iterator.
    fn lex_string<T>(&mut self, mut iter: T, from: usize) -> LexResult
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

                    let remaining = self
                        .source
                        .get(from..take)
                        .expect("valid error must contain range");

                    return Err(Error::build(INVALID_SYNTAX)
                        .pointer(self.source, from..take)
                        .help("this might be an undelimited string, try closing it with `\"`"));
                }
            }
        }
    }

    /// Lex a [Token::Ident | Token::Keyword, Region] with the given iterator.
    ///
    /// If the iterator yields an identifier that matches a recognized keyword,
    /// a Token::Keyword is returned. Otherwise, a Token::Identifier is returned.
    fn lex_ident_or_keyword<T>(&mut self, mut iter: T, from: usize) -> (Token, Region)
    where
        T: Iterator<Item = (usize, char)>,
    {
        let mut check_keyword = |to: usize| {
            let range_text = self
                .source
                .get(from..to)
                .expect("valid range is required to check keyword");

            let token = match range_text.to_lowercase().as_str() {
                "not" => Token::Keyword(Keyword::Not),
                "if" => Token::Keyword(Keyword::If),
                "else" => Token::Keyword(Keyword::Else),
                "endif" => Token::Keyword(Keyword::EndIf),
                "let" => Token::Keyword(Keyword::Let),
                "for" => Token::Keyword(Keyword::For),
                "in" => Token::Keyword(Keyword::In),
                "endfor" => Token::Keyword(Keyword::EndFor),
                "include" => Token::Keyword(Keyword::Include),
                "extends" => Token::Keyword(Keyword::Extends),
                "block" => Token::Keyword(Keyword::Block),
                "endblock" => Token::Keyword(Keyword::EndBlock),
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

    /// Lex a (Token, Region) and behave as if the cursor is not inside of an
    /// expression or block.
    fn lex_default(&mut self, from: usize) -> LexResult {
        // Trims the bounds of the given indices and returns them as a Region.
        let mut trim_region = |mut region_begin, mut region_end, right_trim| {
            if right_trim {
                region_end = self.source[..region_end].trim_end().len();
            }

            if self.left_trim {
                self.left_trim = false;
                let s = &self.source[region_begin..region_end];
                region_begin = s.len() - s.trim_start().len()
            }

            // The only Token kind that should ever be trimmed is Token::Raw,
            // so we just assume that is what this is.
            Ok(Some((Token::Raw, (region_begin..region_end).into())))
        };

        match self.finder.next(self.source, from) {
            Some((id, marker_begin, marker_end)) => {
                // Found a marker, so return everything up to that marker first as
                // Token::Raw and store the marker in buffer.
                let (token, is_trimmed) = Token::from_usize_trim(id);

                match &token {
                    Token::BeginExpression => {
                        self.state = State::Tag {
                            end_token: Token::EndExpression,
                        }
                    }
                    Token::BeginBlock => {
                        self.state = State::Tag {
                            end_token: Token::EndBlock,
                        }
                    }
                    _ => {
                        return Err(Error::build(UNEXPECTED_TOKEN)
                            .pointer(self.source, marker_begin..marker_end)
                            .help("expected beginning expression or beginning block"));
                    }
                }

                if from == marker_begin {
                    // No raw text is between our cursor and the marker, so we can
                    // skip storing it on the buffer and just return it right away.
                    self.cursor = marker_end;

                    Ok(Some((token, (marker_begin..marker_end).into())))
                } else {
                    // Store the marker in buffer.
                    self.cursor = marker_end;
                    self.buffer = Some((token, (marker_begin..marker_end).into()));

                    // Return the Token::Raw pointing to everything up to the
                    // beginning of the marker.
                    trim_region(from, marker_begin, is_trimmed)
                }
            }
            None => {
                // Nothing found. Advance to the end.
                let remaining = self.cursor..self.source.len();
                self.cursor = self.source.len();

                trim_region(from, remaining.end, false)
            }
        }
    }
}

/// Return true if the given character is a recognized beginning identifier,
/// meaning a '_' or `xid_start`.
fn is_ident_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

/// Return true if the given character is a recognized continue identifier,
/// meaning an `xid_continue`.
fn is_ident_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

/// Return true if the given character is a number (0-9) or a period.
///
/// Period is considered to be number because it often appears in floats
/// such as "10.2".
fn is_number(c: char) -> bool {
    matches!(c, '0'..='9' | '.')
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::{
        compile::{
            lex::{State, Token},
            Keyword, Operator,
        },
        log::Error,
        region::Region,
    };
    use std::{
        fmt::Write,
        fs::{read_to_string, File},
        io::{BufRead, BufReader},
    };

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
        let mut block_lexer = Lexer::new("lorem (*");
        let mut expression_lexer = Lexer::new("lorem ((");
        block_lexer.next()?;
        expression_lexer.next()?;
        assert_eq!(
            block_lexer.state,
            State::Tag {
                end_token: Token::EndBlock
            }
        );
        assert_eq!(
            expression_lexer.state,
            State::Tag {
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

        // Lexer should convert to lowercase, so this should be okay.
        helper_lex_next_auto("(( IF ))", expect);
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
    fn bonkers() {
        let mut lexer = Lexer::new("(( 3 ))");
        for i in 0..3 {
            let next = lexer.next();
            println!("{:?}", next)
        }
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
            (Token::Raw, 0..196),
            (Token::BeginExpression, 196..198),
            (Token::Identifier, 199..203),
            (Token::EndExpression, 204..206),
            (Token::Raw, 206..212),
            (Token::BeginBlock, 212..214),
            (Token::Keyword(Keyword::For), 215..218),
            (Token::Identifier, 219..225),
            (Token::Keyword(Keyword::In), 226..228),
            (Token::Identifier, 229..235),
            (Token::EndBlock, 236..238),
            (Token::Raw, 238..247),
            (Token::BeginExpression, 247..249),
            (Token::Identifier, 250..256),
            (Token::Period, 256..257),
            (Token::Identifier, 257..261),
            (Token::EndExpression, 262..264),
            (Token::Raw, 264..269),
            (Token::BeginBlock, 269..271),
            (Token::Keyword(Keyword::EndFor), 272..278),
            (Token::EndBlock, 279..281),
            (Token::Raw, 281..287),
            (Token::BeginBlock, 287..289),
            (Token::Keyword(Keyword::If), 290..292),
            (Token::Identifier, 293..297),
            (Token::Operator(Operator::Equal), 298..300),
            (Token::String, 301..309),
            (Token::EndBlock, 310..312),
            (Token::Raw, 312..347),
            (Token::BeginBlock, 347..349),
            (Token::Keyword(Keyword::EndIf), 350..355),
            (Token::EndBlock, 356..358),
            (Token::Raw, 358..364),
            (Token::BeginExpression, 364..366),
            (Token::Identifier, 367..371),
            (Token::Pipe, 372..373),
            (Token::Identifier, 374..381),
            (Token::Number, 382..383),
            (Token::Colon, 383..384),
            (Token::String, 385..394),
            (Token::Pipe, 395..396),
            (Token::Identifier, 397..403),
            (Token::String, 404..407),
            (Token::Pipe, 408..409),
            (Token::Identifier, 410..415),
            (Token::EndExpression, 416..418),
            (Token::Raw, 418..434),
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

        let mut lexer = Lexer::new("hello (( name (( ))");
        for (token, range) in expect {
            assert_eq!(lexer.next(), Ok(Some((token, range.into()))))
        }

        let next = lexer
            .next()
            .expect_err("should receive err with overlapping tags");

        // println!("{:#}", next);
        assert!(lexer.next().is_err())
    }

    /// Helper function which takes in a source string, creates a lexer on that
    /// string and iterates [expect.len()] amount of times and compares the result
    /// against [lexer.next()].
    fn helper_lex_next_auto<T>(source: &str, expect: Vec<(Token, T)>)
    where
        T: Into<Region>,
    {
        let mut lexer = Lexer::new(source);
        for (token, region) in expect {
            assert_eq!(lexer.next(), Ok(Some((token, region.into()))))
        }

        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
    }
}
