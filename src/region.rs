use crate::{log::INVALID_SYNTAX, Error, Pointer};
use std::{
    cmp::{max, min},
    fmt::Display,
    ops::{Index, Range},
};

/// Represents a region (beginning and ending indices) within some source.
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

    /// Return true if this region ends where the given Region begins,
    /// or this Region begins where the given Region ends.
    pub fn is_neighbor(&self, other: Self) -> bool {
        self.end == other.begin || other.end == self.begin
    }

    /// Return a new Region which represents the area between this Region
    /// and the given Region.
    ///
    /// Returns None if the regions are neighbors.
    pub fn difference(&self, other: Self) -> Option<Self> {
        if self.is_neighbor(other) {
            return None;
        }

        Some(if self.begin < other.begin {
            Self {
                begin: self.end,
                end: other.begin,
            }
        } else {
            Self {
                begin: other.end,
                end: self.begin,
            }
        })
    }

    /// Combine will merge the indices of two Region instances.
    pub fn combine(self, other: Self) -> Self {
        Self {
            begin: min(self.begin, other.begin),
            end: max(self.end, other.end),
        }
    }

    /// Access the literal value of a Region.
    ///
    /// # Errors
    ///
    /// Returns an Error if the Region is out of bounds in the given source text.
    pub fn literal<'source>(&self, source: &'source str) -> Result<&'source str, Error> {
        source.get(self.begin..self.end).ok_or_else(|| {
            Error::build(INVALID_SYNTAX)
                .visual(Pointer::new(source, (self.begin..self.end).into()))
                .help(
                    "unable to locate literal value in source text, was the source modified \
                    after template compilation?",
                )
        })
    }
}

impl Index<Region> for str {
    type Output = str;

    fn index(&self, region: Region) -> &Self::Output {
        let Region { begin, end } = region;
        &self[begin..end]
    }
}

impl Index<Region> for String {
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

impl From<Region> for Range<usize> {
    fn from(value: Region) -> Self {
        Self {
            start: value.begin,
            end: value.end,
        }
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.begin, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_neighbor() {
        let region1 = Region::new(0..3);
        let region2 = Region::new(3..6);
        assert_eq!(region1.difference(region2), None);
    }

    #[test]
    fn test_diff_not_neighbor() {
        let region1 = Region::new(0..3);
        let region2 = Region::new(4..7);
        assert_eq!(region1.difference(region2), Some(Region::new(3..4)));
    }
}
