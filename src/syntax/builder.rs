use scout::Syntax;

/// Delimiters that identify blocks and expressions within templates.
///
/// The actual value of each marker (custom delimiters) can be set by way of the
/// Builder type.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Provides methods to easily build a Syntax.
///
/// The types behind the constructed syntax are described by Marker.
///
/// # Example
///
/// ```
/// use ash::Builder;
/// use scout::Finder;
///
/// let syntax = Builder::new()
///     .expression("((", "))")
///     .block("(*", "*)")
///     .whitespace(&'-')
///     .build();
///
/// let finder = Finder::new(syntax);
/// let result = finder.next("hello ((", 0);
/// ```
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    expression: (&'a str, &'a str),
    block: (&'a str, &'a str),
    whitespace: &'a char,
}

impl<'a> Builder<'a> {
    /// Create a new Builder.
    ///
    /// The delimiters have defaults as described here:
    ///
    /// Expressions: (( name ))
    /// Blocks: (* if ... *)
    /// Whitespace: ((- -)) / (*- -*)
    ///
    /// To proceed with these defaults, you may immediately call 'build'
    /// to receive the Syntax instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            expression: ("((", "))"),
            block: ("(*", "*)"),
            whitespace: &'-',
        }
    }

    /// Set the expression delimiters.
    #[inline]
    pub fn expression(&mut self, begin_expr: &'a str, end_expr: &'a str) -> &mut Self {
        assert!(!begin_expr.is_empty() && !end_expr.is_empty());
        self.expression = (begin_expr, end_expr);
        self
    }

    /// Set the block delimiters.
    #[inline]
    pub fn block(&mut self, begin_block: &'a str, end_block: &'a str) -> &mut Self {
        assert!(!begin_block.is_empty() && !end_block.is_empty());
        self.block = (begin_block, end_block);
        self
    }

    /// Set the whitespace trim character.
    #[inline]
    pub fn whitespace(&mut self, whitespace_tag: &'a char) -> &mut Self {
        assert!(!whitespace_tag.is_whitespace());
        self.whitespace = whitespace_tag;
        self
    }

    /// Build a Syntax instance from the given delimiters.
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
