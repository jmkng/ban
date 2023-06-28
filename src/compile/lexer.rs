use super::token::Token;
use crate::{types::Region, SyntaxBuilder};
use scout::Search;

pub struct Lexer<'source> {
    /// Utility for searching text for patterns.
    finder: Search,
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

    pub fn next(&mut self) -> Option<Region<Token>> {
        // Always prefer taking from the buffer when possible.
        if let Some(next) = self.buffer.take() {
            return Some(next);
        }

        if self.source[self.cursor..].is_empty() {
            return None;
        }

        let c = self.cursor;

        match self.state {
            State::Default => self.lex_default(c),
            State::Tag { .. } => self.lex_tag(c),
        }
    }

    fn lex_tag(&mut self, from: usize) -> Option<Region<Token>> {
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
                            return Some(Region::new(token, from..length));
                        } else {
                            panic!("unexpected token found while lexing tag")
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
                    c if c.is_whitespace() => Some(self.lex_whitespace(iterator, index)),
                    c if c.is_ascii_digit() => Some(self.lex_digit(iterator, index)),
                    c if is_ident_or_keyword(c) => Some(self.lex_ident_or_keyword(iterator, index)),
                    _ => todo!(),
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

    fn lex_string<I>(&mut self, mut iter: I, from: usize) -> Option<Region<Token>>
    where
        I: Iterator<Item = (usize, char)> + Clone,
    {
        let mut curr = '"';
        loop {
            match iter.next() {
                Some((index, '"')) if curr != '\\' => {
                    // Accept a double quote as a signal to end the string, unless the previous
                    // character was an escape.
                    //
                    // Add one to the index of the character to comply with string slice
                    // semantics.
                    let to = index + 1;
                    self.cursor = to;

                    return Some(Region {
                        data: Token::String,
                        position: from..to,
                    });
                }
                Some((_, char)) => {
                    // Assign character to "curr" and move on. We use "curr" to determine
                    // if a double quote should be escaped.
                    //
                    // We are using CharIndices, so no need to keep track of the index.
                    curr = char;
                }
                None => {
                    panic!("found delimited string during lex_string");
                }
            }
        }
    }

    fn lex_ident_or_keyword<I>(&mut self, mut iter: I, from: usize) -> Region<Token>
    where
        I: Iterator<Item = (usize, char)> + Clone,
    {
        loop {
            match iter.next() {
                Some((index, char)) if !is_ident_or_keyword(char) => {
                    self.cursor = index;
                    // TODO: Return Token::Keyword for special cases like "if" / "let" etc..
                    break Region::new(Token::Ident, from..index);
                }
                Some((_, _)) => continue,
                None => break Region::new(Token::Ident, from..self.source.len()),
            }
        }
    }

    fn lex_default(&mut self, from: usize) -> Option<Region<Token>> {
        // Find the next opening expression/block, return everything up to
        // that point as Token::Raw.

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

            Some(Region::new(Token::Raw, region_begin..region_end))
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
                    // TODO: Proper error here instead of panic
                    _ => panic!("unexpected token in top-level scope"),
                }

                if from == marker_begin {
                    // No raw text is between our cursor and the marker, so we can
                    // skip storing it on the buffer and just return it right away.
                    self.cursor = marker_end;
                    Some(Region::new(token, marker_begin..marker_end))
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
        compile::{lexer::State, token::Token},
        types::Region,
    };

    #[test]
    fn test_lex_default_no_match() {
        let source = "lorem ipsum";
        let mut lexer = Lexer::new(source);
        let expect = Region::new(Token::Raw, 0..11);
        assert_eq!(lexer.next(), Some(expect));
        println!("Raw: {:?}", source.get(0..11));
    }

    #[test]
    fn test_lex_default_match_no_trim() {
        let source = "lorem ipsum (( dolor ...";
        let mut lexer = Lexer::new(source);
        let expect = Region::new(Token::Raw, 0..12);
        assert_eq!(lexer.next(), Some(expect));
        // Range should contain whitespace on the right side of "ipsum".
        println!("Raw: {:?}", source.get(0..12));
    }

    #[test]
    fn test_lex_default_match_trim() {
        let source = "lorem ipsum ((- dolor ...";
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next(), Some(Region::new(Token::Raw, 0..11)));
        // Range should not contain whitespace on the right side of "ipsum".
        println!("Raw: {:?}", source.get(0..11));
    }

    #[test]
    fn test_lex_default_match_state() {
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
        println!("Block lexer state: {:?}", block_lexer.state);
        println!("Expression lexer state: {:?}", expression_lexer.state);
    }

    #[test]
    fn test_lex_digit() {
        let source = "(( 10 ))";
        let mut lexer = Lexer::new(source);
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::BeginExpression, 0..2)
        );
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 2..3));
        println!("Number: {:?}", source.get(3..5));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Number, 3..5));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 5..6));
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::EndExpression, 6..8)
        );
    }

    #[test]
    fn test_lex_ident() {
        let source = "(( hello ))";
        let mut lexer = Lexer::new(source);
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::BeginExpression, 0..2)
        );
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 2..3));
        println!("Ident: {:?}", source.get(3..8));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Ident, 3..8));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 8..9));
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::EndExpression, 9..11)
        );
    }

    #[test]
    fn test_lex_string() {
        let source = "(( \"name\" ))";
        let mut lexer = Lexer::new(source);
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::BeginExpression, 0..2)
        );
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 2..3));
        println!("String: {:?}", source.get(3..9));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::String, 3..9));
        assert_eq!(lexer.next().unwrap(), Region::new(Token::Whitespace, 9..10));
        assert_eq!(
            lexer.next().unwrap(),
            Region::new(Token::EndExpression, 10..12)
        );
    }
}
