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
        match self.finder.find_at(self.source, from) {
            Some((id, begin, end)) => {
                // Found a marker, so return everything up to that marker first as
                // Token::Raw.
                //
                // The marker can be stored in the buffer, it will be read next.
                let (token, trim) = Token::from_usize_trim(id);

                match &token {
                    Token::BeginExpression => self.state = State::Expression,
                    Token::BeginBlock => self.state = State::Block,
                    // TODO: Proper error here instead of panic
                    _ => panic!("unexpected token in top-level scope"),
                }

                let mut lex = |token: Token, range: Range<usize>| {
                    self.cursor = range.end;

                    Some(Region::new(token, range))
                };

                if from == begin {
                    // No raw text is between our cursor and the marker, so we can
                    // skip storing it on the buffer and just return it right away.
                    lex(token, begin..end)
                } else {
                    // Store the marker in buffer.
                    self.buffer = lex(token, begin..end);
                    // Return the Token::Raw pointing to everything up to the
                    // beginning of the marker.
                    lex(Token::Raw, from..begin)
                }
            }
            None => {
                // Nothing found. Advance to the end.
                let range = self.cursor..self.source.len();
                self.cursor = self.source.len();

                Some(Region::new(Token::Raw, range))
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
    use crate::{compile::token::Token, types::Region};

    use super::Lexer;

    #[test]
    fn test_lex_default() {
        let mut lexer = Lexer::new("test string");

        let expect = Region::new(Token::Raw, 0..11);

        assert_eq!(lexer.next(), Some(expect))
    }
}
