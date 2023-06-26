use super::token::Token;
use crate::{types::Region, SyntaxBuilder};
use scout::Search;
use std::ops::Range;

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
            State::Block => todo!(),
            State::Expression => todo!(),
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
                    Token::BeginExpression => self.state = State::Expression,
                    Token::BeginBlock => self.state = State::Block,
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

enum State {
    Default,
    Block,
    Expression,
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::{compile::token::Token, types::Region};

    #[test]
    fn test_lex_default_no_match() {
        let mut lexer = Lexer::new("lorem ipsum");
        let expect = Region::new(Token::Raw, 0..11);
        assert_eq!(lexer.next(), Some(expect))
    }

    #[test]
    fn test_lex_default_match_no_trim() {
        let mut lexer = Lexer::new("lorem ipsum (( dolor ...");
        let expect = Region::new(Token::Raw, 0..12);
        assert_eq!(lexer.next(), Some(expect))
    }

    #[test]
    fn test_lex_default_match_trim() {
        let source = "lorem ipsum ((- dolor ...";
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next(), Some(Region::new(Token::Raw, 0..11)))
    }
}
