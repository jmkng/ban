//! Ash lexer.
//!
//! Receives a reference to some external string and provides methods to produce
//! instances of Region, which describe the type and location of tokens within
//! the source.
//!
//! The lexer is designed to be embedded within the parser, rather than used on
//! its own.
//!
//! &str -> Vec<Region> -> Template -> String
//! -------------------
mod state;
mod token;

pub use token::Token;

use self::{state::State, token::Operator};
use crate::{
    compile::lexer::token::Keyword,
    types::{Error, Region},
    SyntaxBuilder,
};
use scout::Finder;

pub type LexResult = Result<Option<(Token, Region)>, Error>;

macro_rules! general_error {
    ($fmt:expr $(, $($args:expr),*)?) => {
        Err(Error::General(format!($fmt $(, $($args),*)?)))
    };
}

pub struct Lexer<'source> {
    /// Utility for searching text for patterns.
    finder: Finder<&'source str>,
    /// Text that is being analyzed.
    source: &'source str,
    /// State determines the action taken when [.next()] is called.
    state: State,
    /// Position within source.
    cursor: usize,
    /// When true, the following Token pulled in State::Default state
    /// will be left trimmed.
    left_trim: bool,
    /// Temporary storage for a "next" token that will be read
    /// on the following call to [.next()]
    buffer: Option<(Token, Region)>,
}

impl<'source> Lexer<'source> {
    /// Create a new instance of Lexer.
    pub fn new(source: &'source str) -> Self {
        Self {
            finder: Finder::new(SyntaxBuilder::new().build()),
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

    /// Lex the next token, behaving as though the cursor is inside of a tag.
    ///
    /// Tag is a general term which can mean either block or expression.
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

                            return Ok(Some((token, (from..length).into())));
                        } else {
                            let which = if *end_token == Token::EndExpression {
                                "expression"
                            } else {
                                "block"
                            };

                            let position = self.get_position();
                            return general_error!(
                                "unexpected token `{}` found at line {} character {}, have you closed the previous {}?",
                                token,
                                position.0,
                                position.1,
                                which
                            );
                        }
                    }
                    _ => panic!("unexpected state for lex_tag"),
                }
            }
            None => {
                // An iterator over the remaining source, with the index adjusted to match the
                // true position based on our "from" parameter.
                let mut i = self.source[from..]
                    .char_indices()
                    .map(|(d, c)| (from + d, c));

                let mut advance = |length: usize, data: Token| {
                    self.cursor += length;
                    Ok(Some((data, (from..from + length).into())))
                };

                let (index, char) = i.next().unwrap();

                match char {
                    '*' => advance(1, Token::Operator(Operator::Multiply)),
                    '+' => advance(1, Token::Operator(Operator::Add)),
                    '/' => advance(1, Token::Operator(Operator::Divide)),
                    '-' => advance(1, Token::Operator(Operator::Subtract)),
                    '.' => advance(1, Token::Period),

                    c if c.is_whitespace() => Ok(Some(self.lex_whitespace(i, index))),
                    c if c.is_ascii_digit() => Ok(Some(self.lex_digit(i, index))),
                    c if is_ident_or_keyword(c) => Ok(Some(self.lex_ident_or_keyword(i, index))),

                    '"' => self.lex_string(i, index),

                    // These operators may have a secondary character to them. ("==" | "!=", etc)
                    // lex_operator will handle it.
                    '=' | '!' | '>' | '<' => self.lex_operator(i, index, char),

                    _ => general_error!("encountered unexpected character `{}`", char),
                }
            }
        }
    }

    /// Lex a Region containing a Token::Operator.
    ///
    /// This function will catch and return these types, but return an error if one of
    /// the patterns is not matched.
    ///
    /// Assign: =
    ///
    /// Equal: ==
    ///
    /// Not Equal: !=
    ///
    /// Greater Or Equal: >=
    ///
    /// Lesser Or Equal: <=
    fn lex_operator<T>(&mut self, mut iter: T, from: usize, previous: char) -> LexResult
    where
        T: Iterator<Item = (usize, char)>,
    {
        let (position, operator) = match (previous, iter.next()) {
            ('=', Some((usize, '='))) => (usize, Operator::Equal),
            ('!', Some((usize, '='))) => (usize, Operator::NotEqual),
            ('>', Some((usize, '='))) => (usize, Operator::GreaterOrEqual),
            ('<', Some((usize, '='))) => (usize, Operator::LesserOrEqual),
            ('=', Some(_)) | ('=', None) => (from, Operator::Assign),
            c if c.1.is_some() => {
                return general_error!("unrecognized operators `{}{}`", previous, c.1.unwrap().1)
            }
            _ => return general_error!("unrecognized operator `{}`", previous),
        };
        let position = position + 1;

        self.cursor = position;
        Ok(Some((Token::Operator(operator), (from..position).into())))
    }

    /// Lex a Region containing a Token::Number.
    fn lex_digit<T>(&mut self, mut iter: T, from: usize) -> (Token, Region)
    where
        T: Iterator<Item = (usize, char)>,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !char.is_ascii_digit() => {
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

    /// Lex a Region containing a Token::String.
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

                    return general_error!(
                        "found undelimited string: `{} ...` <- try adding `\"`",
                        remaining
                    );
                }
            }
        }
    }

    /// Lex a Region containing a Token::Ident or Token::Keyword.
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
                "if" => Token::Keyword(Keyword::If),
                "let" => Token::Keyword(Keyword::Let),
                "for" => Token::Keyword(Keyword::For),
                "in" => Token::Keyword(Keyword::In),
                "include" => Token::Keyword(Keyword::Include),
                "endfor" => Token::Keyword(Keyword::EndFor),
                "endif" => Token::Keyword(Keyword::EndIf),

                _ => Token::Ident,
            };
            self.cursor = to;

            (token, (from..to).into())
        };

        loop {
            match iter.next() {
                Some((index, char)) if !is_ident_or_keyword(char) => {
                    break check_keyword(index);
                }
                Some((_, _)) => continue,
                None => break check_keyword(self.source.len()),
            }
        }
    }

    /// Lex a Region containing a Token::Raw.
    fn lex_default(&mut self, from: usize) -> LexResult {
        // A closure which trims the bounds of the given indices and returns them
        // as a Region, with a Token::Raw embedded in the data property.
        //
        // The only Token type which should ever be trimmed is Token::Raw.
        let mut trim_region = |mut region_begin, mut region_end, right_trim| {
            if right_trim {
                region_end = self.source[..region_end].trim_end().len();
            }

            if self.left_trim {
                self.left_trim = false;
                let s = &self.source[region_begin..region_end];
                region_begin = s.len() - s.trim_start().len()
            }

            Ok(Some((Token::Raw, (region_begin..region_end).into())))
        };

        match self.finder.next(self.source, from) {
            Some((id, marker_begin, marker_end)) => {
                // Found a marker, so return everything up to that marker first as
                // Token::Raw.
                //
                // The marker can be stored in the buffer, it will be read next.
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
                        return general_error!(
                            "unexpected token `{}`, expected beginning expression or beginning block",
                            token
                        );
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

    /// Get the line and column numbers based on the Lexer position.
    fn get_position(&self) -> (usize, usize) {
        (
            calc_line_number(self.source, self.cursor),
            calc_column_number(self.source, self.cursor),
        )
    }
}

/// Determine the amount of newlines that exist in the given source from the beginning
/// (0) up to the given index.
fn calc_line_number(source: &str, index: usize) -> usize {
    let mut line = 1;

    let window = source
        .get(..index)
        .expect("getting window for line calculation should not fail");

    for i in window.chars() {
        if i == '\n' {
            line += 1;
        }
    }

    return line;
}

/// Determine the amount of characters that exist between the given index and the
/// closest newline.
fn calc_column_number(source: &str, index: usize) -> usize {
    let window = source
        .get(..index)
        .expect("getting window for column calculation should not fail");

    let mut chars = 1;
    let mut iterator = window.chars().rev();
    while let Some(c) = iterator.next() {
        if c == '\n' {
            break;
        }
        chars += 1;
    }

    chars
}

/// Checks if the char is a recognized ident/keyword character.
fn is_ident_or_keyword(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '_')
}

#[cfg(test)]
mod tests {
    use super::{
        token::{Keyword, Operator},
        Lexer,
    };
    use crate::{
        compile::lexer::{calc_column_number, calc_line_number, State, Token},
        types::{Error, Region},
    };
    use std::fs::read_to_string;

    #[test]
    fn test_calc_line_number() {
        assert_eq!(calc_line_number("hello\r\nworld\ngoodbye", 4), 1);
        assert_eq!(calc_line_number("hello\r\nworld\ngoodbye", 5), 1);
        assert_eq!(calc_line_number("hello\r\nworld\ngoodbye", 6), 1);
        assert_eq!(calc_line_number("hello\r\nworld\ngoodbye", 7), 2);
        assert_eq!(calc_line_number("hello\r\nworld\ngoodbye", 13), 3);
    }

    #[test]
    fn test_calc_column_number() {
        assert_eq!(calc_column_number("hello\r\nworld\ngoodbye", 6), 7);
        assert_eq!(calc_column_number("hello\r\nworld\ngoodbye", 9), 3);
        assert_eq!(calc_column_number("hello\r\nworld\ngoodbye", 12), 6);
        assert_eq!(calc_column_number("hello\r\nworld\ngoodbye", 13), 1);
        assert_eq!(calc_column_number("hello\r\nworld\ngoodbye", 15), 3);
    }

    #[test]
    fn test_lex_default_no_match() {
        let expect = vec![(Token::Raw, 0..11)];
        lex_next_auto("lorem ipsum", expect)
    }

    #[test]
    fn test_lex_default_match_no_trim() {
        let expect = vec![
            (Token::Raw, 0..12),
            (Token::BeginExpression, 12..14),
            (Token::Ident, 15..20),
        ];
        lex_next_auto("lorem ipsum (( dolor", expect);
    }

    #[test]
    fn test_lex_default_match_trim() {
        let expect = vec![
            (Token::Raw, 0..11),
            (Token::BeginExpression, 12..15),
            (Token::Ident, 16..21),
        ];
        lex_next_auto("lorem ipsum ((- dolor", expect);
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

        lex_next_auto("(( 10 ))", expect);
    }

    #[test]
    fn test_lex_ident() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::Ident, 3..8),
            (Token::EndExpression, 9..11),
        ];

        lex_next_auto("(( hello ))", expect);
    }

    #[test]
    fn test_lex_keyword() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::Keyword(Keyword::If), 3..5),
            (Token::EndExpression, 6..8),
        ];

        // Lexer should convert to lowercase, so this should be okay.
        lex_next_auto("(( IF ))", expect);
    }

    #[test]
    fn test_lex_string_escape() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::String, 3..13),
            (Token::EndExpression, 14..16),
        ];

        lex_next_auto(r#"(( "\"name\"" ))"#, expect);
    }

    #[test]
    fn test_lex_full_document() {
        let source = read_to_string("tests/template.html").unwrap();
        let expect = vec![
            (Token::Raw, 0..196),
            (Token::BeginExpression, 196..198),
            (Token::Ident, 199..203),
            (Token::EndExpression, 204..206),
            (Token::Raw, 206..212),
            (Token::BeginBlock, 212..214),
            (Token::Keyword(Keyword::For), 215..218),
            (Token::Ident, 219..225),
            (Token::Keyword(Keyword::In), 226..228),
            (Token::Ident, 229..235),
            (Token::EndBlock, 236..238),
            (Token::Raw, 238..247),
            (Token::BeginExpression, 247..249),
            (Token::Ident, 250..256),
            (Token::Period, 256..257),
            (Token::Ident, 257..261),
            (Token::EndExpression, 262..264),
            (Token::Raw, 264..269),
            (Token::BeginBlock, 269..271),
            (Token::Keyword(Keyword::EndFor), 272..278),
            (Token::EndBlock, 279..281),
            (Token::Raw, 281..287),
            (Token::BeginBlock, 287..289),
            (Token::Keyword(Keyword::If), 290..292),
            (Token::Ident, 293..297),
            (Token::Operator(Operator::Equal), 298..300),
            (Token::String, 301..309),
            (Token::EndBlock, 310..312),
            (Token::Raw, 312..347),
            (Token::BeginBlock, 347..349),
            (Token::Keyword(Keyword::EndFor), 350..356),
            (Token::EndBlock, 357..359),
            (Token::Raw, 359..375),
        ];

        lex_next_auto(&source, expect)
    }

    #[test]
    fn test_lex_string() {
        let expect = vec![
            (Token::BeginExpression, 0..2),
            (Token::String, 3..9),
            (Token::EndExpression, 10..12),
        ];

        lex_next_auto("(( \"name\" ))", expect);
    }

    #[test]
    fn test_error_multiple_opening_tags() {
        let expect = vec![
            (Token::Raw, 0..6),
            (Token::BeginExpression, 6..8),
            (Token::Ident, 9..13),
        ];
        let mut lexer = Lexer::new("hello (( name (( ))");
        // Expecting error, so cannot use lex_next_auto
        for (token, range) in expect {
            assert_eq!(lexer.next(), Ok(Some((token, range.into()))))
        }
        let next = lexer
            .next()
            .expect_err("should receive err with overlapping tags");

        assert_eq!(next.to_string(), "unexpected token `begin expression` found at line 1 character 15, have you closed the previous expression?");
        assert!(lexer.next().is_err())
    }

    /// Helper function which takes in a source string, creates a lexer on that
    /// string and iterates [expect.len()] amount of times and compares the result
    /// against [lexer.next()].
    fn lex_next_auto<T>(source: &str, expect: Vec<(Token, T)>)
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
