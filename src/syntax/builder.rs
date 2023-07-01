use scout::Syntax;

/// Defines the delimiters that identify blocks and expressions within templates.
///
/// The actual value of each marker (custom delimiters) can be set by way of the
/// SyntaxBuilder type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    BeginExpression = 0,
    EndExpression = 1,
    BeginExpressionTrim = 2,
    EndExpressionTrim = 3,
    BeginBlock = 4,
    EndBlock = 5,
    BeginBlockTrim = 6,
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

/// A handy way to build a new instance of Syntax.
///
/// The types behind the constructed syntax are described by syntax::Marker.
///
/// # Example
///
/// ```
/// use ash::SyntaxBuilder;
/// use scout::Finder;
///
/// let syntax = SyntaxBuilder::new()
///     .expression("((", "))")
///     .block("(*", "*)")
///     .whitespace(&'-')
///     .build();
///
/// let finder = Finder::new(syntax);
/// let result = finder.next("hello ((", 0);
/// ```
#[derive(Debug, Clone)]
pub struct SyntaxBuilder<'a> {
    expression: (&'a str, &'a str),
    block: (&'a str, &'a str),
    whitespace: &'a char,
}

impl<'a> SyntaxBuilder<'a> {
    /// Create a new SyntaxBuilder.
    ///
    /// The delimiters have defaults as described here:
    ///
    /// Expressions: (( name ))
    /// Blocks: (* if ... *)
    /// Whitespace: ((- -)) / (*- -*)
    ///
    /// To proceed with these defaults, you may immediately call [.build()]
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
