use std::{
    cmp::{max, min},
    ops::{Index, Range},
};

/// Represents an area within source text.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Region {
    /// The beginning of the range, inclusive.
    pub begin: usize,
    /// The ending of the range, exclusive.
    pub end: usize,
}

impl Region {
    /// Create a new Region from the given range.
    pub fn new(position: Range<usize>) -> Self {
        Self {
            begin: position.start,
            end: position.end,
        }
    }

    /// Return true if this [`Region`] ends where the given `Region` begins,
    /// or this `Region` begins where the given `Region` ends.
    pub fn is_neighbor(&self, other: Self) -> bool {
        self.end == other.begin || other.end == self.begin
    }

    /// Combine will merge the indices of two [`Region`] instances.
    pub fn combine(self, other: Self) -> Self {
        Self {
            begin: min(self.begin, other.begin),
            end: max(self.end, other.end),
        }
    }

    /// Access the literal value of a [`Region`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the `Region` is out of bounds in the given source text.
    pub fn literal<'source>(&self, source: &'source str) -> &'source str {
        let literal = source
            .get(self.begin..self.end)
            .expect("getting literal by region should not fail");

        literal
    }
}

impl Index<Region> for str {
    type Output = str;

    fn index(&self, region: Region) -> &Self::Output {
        let Region { begin, end } = region;

        &self[begin..end]
    }
}

impl From<Range<usize>> for Region {
    fn from(value: Range<usize>) -> Self {
        Self {
            begin: value.start,
            end: value.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_neighbor() {
        assert!(Region::new(0..5).is_neighbor(Region::new(5..10)));
        assert!(!Region::new(5..10).is_neighbor(Region::new(11..14)));
    }

    #[test]
    fn test_combine() {
        let combined = Region::new(5..10).combine(Region::new(8..15));

        assert_eq!(combined.begin, 5);
        assert_eq!(combined.end, 15);
    }

    #[test]
    fn test_literal() {
        let source = "Hello, Taylor!";
        let region = Region::new(7..13);

        assert_eq!(region.literal(source), "Taylor");
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds_literal() {
        let source = "Hello, Taylor!";
        let region = Region::new(7..15);

        region.literal(source);
    }
}
