use super::token::{Keyword, Token};
use crate::{
    types::{Error, LexResult, Region},
    SyntaxBuilder,
};
use scout::Search;

pub struct Lexer<'source> {
    /// Utility for searching text for patterns.
    finder: Search,
    /// Text that is being analyzed.
    source: &'source str,
    ///

    /// State determines the action taken when [.next()] is called.
    state: State,
    /// Position within source.
    cursor: usize,
    /// When true, the following Token pulled in State::Default state
    /// will be left trimmed.
    left_trim: bool,
    /// Temporary storage for a "next" token that will be read
    /// on the following call to [.next()]
    buffer: Option<Region<Token>>,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            finder: Search::new(SyntaxBuilder::new().build()),
            state: State::Default,
            source,
            left_trim: false,
            cursor: 0,
            buffer: None,
        }
    }

    pub fn next(&mut self) -> LexResult {
        // Always prefer taking from the buffer when possible.
        if let Some(next) = self.buffer.take() {
            return Ok(Some(next));
        }

        if self.source[self.cursor..].is_empty() {
            return Ok(None);
        }

        let c = self.cursor;

        match self.state {
            State::Default => self.lex_default(c),
            State::Tag { .. } => self.lex_tag(c),
        }
    }

    fn lex_tag(&mut self, from: usize) -> LexResult {
        match self.finder.starts_with(self.source, from) {
            Some((id, length)) => {
                let (token, is_trimmed) = Token::from_usize_trim(id);

                match self.state {
                    State::Tag { ref end_token } => {
                        // Matching end.
                        if token == *end_token {
                            self.state = State::Default;
                            self.left_trim = is_trimmed;
                            self.cursor = length;
                            return Ok(Some(Region::new(token, from..length)));
                        } else {
                            let which = if *end_token == Token::EndExpression {
                                "expression"
                            } else {
                                "block"
                            };

                            let range = self.source.get(from..length);
                            assert_ne!(range, None, "valid error must contain range");

                            let message = format!(
                                "unexpected token `{token}` found at {}, \
                                have you closed the {which} with `{end_token}`?",
                                range.unwrap()
                            );

                            return Err(Error::Lex(message));
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

                let (index, char) = iterator.next().unwrap();

                return match char {
                    '"' => self.lex_string(iterator, index), // which tokens should we care about?
                    c if c.is_whitespace() => Ok(Some(self.lex_whitespace(iterator, index))),
                    c if c.is_ascii_digit() => Ok(Some(self.lex_digit(iterator, index))),
                    c if is_ident_or_keyword(c) => {
                        Ok(Some(self.lex_ident_or_keyword(iterator, index)))
                    }
                    // TODO: temporary error message, should be made more useful as
                    // implementation grows.
                    _ => Err(Error::Lex(format!(
                        "encountered unexpected character `{char}`"
                    ))),
                };
            }
        }
    }

    fn lex_digit<I>(&mut self, mut iter: I, from: usize) -> Region<Token>
    where
        I: Iterator<Item = (usize, char)>,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !char.is_ascii_digit() => {
                    self.cursor = index;
                    break Region::new(Token::Number, from..index);
                }
                Some((_, _)) => continue,
                None => return Region::new(Token::Number, from..self.source.len()),
            }
        }
    }

    fn lex_whitespace<I>(&mut self, mut iter: I, from: usize) -> Region<Token>
    where
        I: Iterator<Item = (usize, char)>,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !char.is_whitespace() => {
                    self.cursor = index;
                    break Region::new(Token::Whitespace, from..index);
                }
                Some((_, _)) => continue,
                None => return Region::new(Token::Whitespace, from..self.source.len()),
            }
        }
    }

    fn lex_string<I>(&mut self, mut iter: I, from: usize) -> LexResult
    where
        I: Iterator<Item = (usize, char)> + Clone,
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

                    return Ok(Some(Region {
                        data: Token::String,
                        position: from..to,
                    }));
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
                    // Unwrap should be safe due to range check above.
                    let remaining = self.source.get(from..take);
                    assert_ne!(remaining, None, "valid error must contain range");
                    let message = format!(
                        "found undelimited string: `{} ...` <- try adding `\"` ",
                        remaining.unwrap()
                    );
                    return Err(Error::Lex(message));
                }
            }
        }
    }

    fn lex_ident_or_keyword<I>(&mut self, mut iter: I, from: usize) -> Region<Token>
    where
        I: Iterator<Item = (usize, char)> + Clone,
    {
        let check_keyword = |to: usize| {
            let range_text = self.source.get(from..to);
            assert_ne!(range_text, None, "valid range is required to check keyword");

            match range_text.unwrap() {
                "if" => Region::new(Token::Keyword(Keyword::If), from..to),
                "let" => Region::new(Token::Keyword(Keyword::Let), from..to),
                "for" => Region::new(Token::Keyword(Keyword::For), from..to),
                "include" => Region::new(Token::Keyword(Keyword::Include), from..to),
                _ => Region::new(Token::Ident, from..to),
            }
        };

        loop {
            match iter.next() {
                Some((index, char)) if !is_ident_or_keyword(char) => {
                    self.cursor = index;
                    // TODO: Return Token::Keyword for special cases like "if" / "let" etc..
                    break check_keyword(index);
                }
                Some((_, _)) => continue,
                None => break check_keyword(self.source.len()),
            }
        }
    }

    /// Begin lexing for State::Default.
    ///
    /// Scans ahead until a BeginExpression or BeginBlock marker is found.
    /// A Region containing a Token::Raw pointing to all of the text up to that marker
    /// is returned.
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

            let region = region_begin..region_end;
            Ok(Some(Region::new(Token::Raw, region)))
        };

        match self.finder.find_at(self.source, from) {
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
                        let message = format!(
                            "unexpected token `{token}`, expected beginning\
                            expression or beginning block",
                        );
                        return Err(Error::Lex(message));
                    }
                }

                if from == marker_begin {
                    // No raw text is between our cursor and the marker, so we can
                    // skip storing it on the buffer and just return it right away.
                    self.cursor = marker_end;
                    Ok(Some(Region::new(token, marker_begin..marker_end)))
                } else {
                    // Store the marker in buffer.
                    self.cursor = marker_end;
                    self.buffer = Some(Region::new(token, marker_begin..marker_end));

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

fn is_ident_or_keyword(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '_')
}

#[derive(Debug, PartialEq)]
enum State {
    Default,
    /// Tag refers to a Block or Expression.
    Tag {
        /// The expected end Token to pair with this Tag.
        /// Either "))" or "*)" by default.
        end_token: Token,
    },
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::{
        compile::{
            lexer::State,
            token::{Keyword, Token},
        },
        types::Region,
    };

    #[test]
    fn test_lex_default_no_match() {
        let source = "lorem ipsum";
        let mut lexer = Lexer::new(source);
        let expect = Region::new(Token::Raw, 0..11);
        assert_eq!(lexer.next(), Ok(Some(expect)));
        println!("Raw: {:?}", source.get(0..11));
    }

    #[test]
    fn test_lex_default_match_no_trim() {
        let source = "lorem ipsum (( dolor ...";
        let mut lexer = Lexer::new(source);
        let expect = Region::new(Token::Raw, 0..12);
        assert_eq!(lexer.next(), Ok(Some(expect)));
        // Range should contain whitespace on the right side of "ipsum".
        println!("Raw: {:?}", source.get(0..12));
    }

    #[test]
    fn test_lex_default_match_trim() {
        let source = "lorem ipsum ((- dolor ...";
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next(), Ok(Some(Region::new(Token::Raw, 0..11))));
        // Range should not contain whitespace on the right side of "ipsum".
        println!("Raw: {:?}", source.get(0..11));
    }

    #[test]
    fn test_lex_state_change() {
        let mut block_lexer = Lexer::new("lorem (*");
        let mut expression_lexer = Lexer::new("lorem ((");
        block_lexer.next();
        expression_lexer.next();
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
    }

    #[test]
    fn test_lex_digit() {
        let expect = vec![
            Region::new(Token::BeginExpression, 0..2),
            Region::new(Token::Whitespace, 2..3),
            Region::new(Token::Number, 3..5),
            Region::new(Token::Whitespace, 5..6),
            Region::new(Token::EndExpression, 6..8),
        ];

        lex_next_auto("(( 10 ))", expect);
    }

    #[test]
    fn test_lex_ident() {
        let expect = vec![
            Region::new(Token::BeginExpression, 0..2),
            Region::new(Token::Whitespace, 2..3),
            Region::new(Token::Ident, 3..8),
            Region::new(Token::Whitespace, 8..9),
            Region::new(Token::EndExpression, 9..11),
        ];

        lex_next_auto("(( hello ))", expect);
    }

    #[test]
    fn test_lex_keyword() {
        let expect = vec![
            Region::new(Token::BeginExpression, 0..2),
            Region::new(Token::Whitespace, 2..3),
            Region::new(Token::Keyword(Keyword::If), 3..5),
            Region::new(Token::Whitespace, 5..6),
            Region::new(Token::EndExpression, 6..8),
        ];

        lex_next_auto("(( if ))", expect);
    }

    #[test]
    fn test_lex_string() {
        let expect = vec![
            Region::new(Token::BeginExpression, 0..2),
            Region::new(Token::Whitespace, 2..3),
            Region::new(Token::String, 3..9),
            Region::new(Token::Whitespace, 9..10),
            Region::new(Token::EndExpression, 10..12),
        ];

        lex_next_auto("(( \"name\" ))", expect);
    }

    /// Helper function which takes in a source string, creates a lexer on that
    /// string and iterates [expect.len()] amount of times and compares the result
    /// against [lexer.next()].
    fn lex_next_auto(source: &str, expect: Vec<Region<Token>>) {
        let mut lexer = Lexer::new(source);
        for i in expect {
            assert_eq!(lexer.next(), Ok(Some(i)))
        }

        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
        assert_eq!(lexer.next(), Ok(None));
    }
}
