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
/// use ash::{Marker, SyntaxBuilder};
/// use scout::Search;
///
/// let syntax = SyntaxBuilder::new()
///     .expression("((", "))")
///     .block("(*", "*)")
///     .whitespace(&'-')
///     .build();
///
/// let search = Search::new(syntax);
/// let result = search.find_at("hello ((", 0);
///
/// assert_eq!(result, Some((Marker::BeginExpression as usize, 6, 8)));
/// ```
#[derive(Debug, Clone)]
pub struct SyntaxBuilder<'a> {
    expression: Option<(&'a str, &'a str)>,
    block: Option<(&'a str, &'a str)>,
    whitespace: &'a char,
}

impl<'a> SyntaxBuilder<'a> {
    /// Create a new SyntaxBuilder.
    #[inline]
    pub fn new() -> Self {
        Self {
            expression: None,
            block: None,
            whitespace: &'-',
        }
    }

    /// Create a new instance of Syntax according to the Ash defaults.
    ///
    /// Expressions: (( name ))
    /// Blocks: (* if ... *)
    /// Whitespace: ((- -)) / (*- -*)
    #[inline]
    pub fn default_ash_syntax() -> Syntax {
        SyntaxBuilder::new()
            .expression("((", "))")
            .block("(*", "*)")
            .whitespace(&'-')
            .build()
    }

    /// Set the expression delimiters.
    #[inline]
    pub fn expression(&mut self, begin_expr: &'a str, end_expr: &'a str) -> &mut Self {
        assert!(!begin_expr.is_empty() && !end_expr.is_empty());
        self.expression = Some((begin_expr, end_expr));

        self
    }

    /// Set the block delimiters.
    #[inline]
    pub fn block(&mut self, begin_block: &'a str, end_block: &'a str) -> &mut Self {
        assert!(!begin_block.is_empty() && !end_block.is_empty());
        self.block = Some((begin_block, end_block));

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
        let ws = self.whitespace;
        if let Some((begin, end)) = self.expression {
            markers.push((Marker::BeginExpression as usize, begin.into()));
            markers.push((Marker::EndExpression as usize, end.into()));
            markers.push((Marker::BeginExpressionTrim as usize, format!("{begin}{ws}")));
            markers.push((Marker::EndExpressionTrim as usize, format!("{ws}{end}")));
        };
        if let Some((begin, end)) = self.block {
            markers.push((Marker::BeginBlock as usize, begin.into()));
            markers.push((Marker::EndBlock as usize, end.into()));
            markers.push((Marker::BeginBlockTrim as usize, format!("{begin}{ws}")));
            markers.push((Marker::EndBlockTrim as usize, format!("{ws}{end}")));
        }

        Syntax::new(markers)
    }
}
