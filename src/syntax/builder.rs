use morel::Syntax;

/// Markers that identify blocks and expressions within text.
///
/// The actual value of each marker (custom delimiters) can be set by way of the
/// [`Builder`][`crate::Builder`] type.
pub enum Marker {
    /// Beginning of an Expression, which allows for outputting content
    /// and passing data through filters.
    BeginExpression = 0,
    /// End of an Expression.
    EndExpression = 1,
    /// Same as BeginExpression, but causes the trailing whitespace of the
    /// preceding raw text to be removed.
    BeginExpressionTrim = 2,
    /// Same as EndExpression, but causes the leading whitespace of the
    /// following raw text to be removed.
    EndExpressionTrim = 3,
    /// Beginning of a Block, which allows for logical constructs such
    /// as "if", "let" and "for".
    BeginBlock = 4,
    /// End of a Block.
    EndBlock = 5,
    /// Same as BeginBlock, but causes the trailing whitespace of the
    /// preceding raw text to be removed.
    BeginBlockTrim = 6,
    /// Same as EndBlock, but causes the leading whitespace of the
    /// following raw text to be removed.
    EndBlockTrim = 7,
}

impl From<usize> for Marker {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::BeginExpression,
            1 => Self::EndExpression,
            2 => Self::BeginExpressionTrim,
            3 => Self::EndExpressionTrim,
            4 => Self::BeginBlock,
            5 => Self::EndBlock,
            6 => Self::BeginBlockTrim,
            7 => Self::EndBlockTrim,
            _ => unreachable!(),
        }
    }
}

impl From<Marker> for usize {
    fn from(k: Marker) -> Self {
        k as usize
    }
}

/// Provides methods to build a Syntax.
///
/// # Example
///
/// ```
/// use ban::Builder;
///
/// let syntax = Builder::new()
///     .with_expression("{{", "}}")
///     .with_block("{*", "*}")
///     .build();
/// ```
pub struct Builder<'marker> {
    expression: (&'marker str, &'marker str),
    block: (&'marker str, &'marker str),
    whitespace: &'marker char,
}

impl<'marker> Builder<'marker> {
    /// Create a new Builder.
    ///
    /// The Builder has default markers:
    ///
    /// ```text
    /// Expressions: (( name ))
    /// Blocks: (* if ... *)
    /// Whitespace:
    ///     Expression: ((- name -))
    ///     Block:  (*- if ... -*)
    /// ```
    ///
    /// To proceed with these defaults, you may immediately call `build`
    /// to receive the Syntax instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            expression: ("((", "))"),
            block: ("(*", "*)"),
            whitespace: &'-',
        }
    }

    /// Set the expression markers.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// let mut builder = Builder::new();
    /// builder.set_expression("{{", "}}");
    /// ```
    #[inline]
    pub fn set_expression(&mut self, begin: &'marker str, end: &'marker str) {
        self.expression = (begin, end);
    }

    /// Set the expression markers.
    ///
    /// Returns the Builder, so additional methods may be chained.
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// Builder::new()
    ///     .with_expression("{{", "}}");
    /// ```
    #[inline]
    pub fn with_expression(mut self, begin: &'marker str, end: &'marker str) -> Self {
        self.set_expression(begin, end);
        self
    }

    /// Set the block markers.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// let mut builder = Builder::new();
    /// builder.set_block("{*", "*}");
    /// ```
    #[inline]
    pub fn set_block(&mut self, begin: &'marker str, end: &'marker str) {
        self.block = (begin, end);
    }

    /// Set the block markers.
    ///
    /// Returns the Builder, so additional methods may be chained.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// Builder::new()
    ///     .with_block("{*", "*}");
    /// ```
    #[inline]
    pub fn with_block(mut self, begin: &'marker str, end: &'marker str) -> Self {
        self.set_block(begin, end);
        self
    }

    /// Set the whitespace trim character.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// let mut builder = Builder::new();
    /// builder.set_whitespace(&'!');
    /// ```
    #[inline]
    pub fn set_whitespace(&mut self, character: &'marker char) {
        self.whitespace = character;
    }

    /// Set the whitespace trim character.
    ///
    /// Returns the Builder, so additional methods may be chained.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// Builder::new()
    ///     .with_whitespace(&'!');
    /// ```
    #[inline]
    pub fn with_whitespace(mut self, character: &'marker char) -> Self {
        self.set_whitespace(character);
        self
    }

    /// Return a Syntax instance from the markers in this Builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Builder;
    ///
    /// let syntax = Builder::new()
    ///     .with_expression("{{", "}}")
    ///     .with_block("{*", "*}")
    ///     .with_whitespace(&'!')
    ///     .build();
    /// ```
    pub fn build(&self) -> Syntax {
        let mut markers = Vec::new();

        let (ex0, ex1) = self.expression;
        let (bl0, bl1) = self.block;
        let ws = self.whitespace;

        markers.push((Marker::BeginExpression.into(), ex0.into()));
        markers.push((Marker::EndExpression.into(), ex1.into()));
        markers.push((Marker::BeginExpressionTrim.into(), format!("{ex0}{ws}")));
        markers.push((Marker::EndExpressionTrim.into(), format!("{ws}{ex1}")));
        markers.push((Marker::BeginBlock.into(), bl0.into()));
        markers.push((Marker::EndBlock.into(), bl1.into()));
        markers.push((Marker::BeginBlockTrim.into(), format!("{bl0}{ws}")));
        markers.push((Marker::EndBlockTrim.into(), format!("{ws}{bl1}")));

        Syntax::new(markers)
    }
}

#[cfg(test)]
mod tests {
    use crate::Builder;
    use morel::Finder;

    #[test]
    fn test_default() {
        let syntax = Builder::new().build();
        let result = Finder::new(syntax).next("hello (( there", 0);

        assert!(result.is_some());
        assert_eq!(result.unwrap().1, 6);
        assert_eq!(result.unwrap().2, 8);
    }
}
