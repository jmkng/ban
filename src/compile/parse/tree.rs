use crate::{
    compile::{Operator, Scope},
    region::Region,
};
use serde_json::Value;

/// The Abstract Syntax Tree.
#[derive(Debug, Clone)]
pub enum Tree {
    /// Raw text.
    Raw(Region),
    /// Render a variable.
    Output(Output),
    /// Render another template.
    Include(Include),
    /// An if or else if expression.
    IfElse(IfElse),
    /// A for loop.
    ForLoop(ForLoop),
}

/// Represents data within expression tags, "(( ))" by default, and may be a Base
/// or Call variant.
#[derive(Debug, Clone)]
pub enum Expression {
    /// A Simple call to render the named value from the Store.
    Base(Base),
    /// A more complex variant which typically retrieves some value from the Store and modifies
    /// it with functions before rendering.
    ///
    /// May also operate on a string literal.
    Call(Call),
}

impl Expression {
    /// Get the Region from the underlying Expression kind.
    pub fn get_region(&self) -> Region {
        match self {
            Expression::Base(base) => base.get_region(),
            Expression::Call(call) => call.region,
        }
    }
}

/// Storage for a "stack" of [`Path`] instances.
///
/// For example, this example represents one [`Compare`] made up of two [`Path`]
/// instances, which are separated at the "||" characters:
///
/// this > that && those == these || they > them
#[derive(Debug)]
pub struct Compare {
    pub paths: Vec<Path>,
}

impl Compare {
    /// Create a new [`Compare`] with one initialized [`Path`].
    pub fn new() -> Self {
        Self {
            paths: vec![Path::new()],
        }
    }

    /// Split up the [`Compare`] by adding a new [`Path`].
    ///
    /// Any additional [`Check`] instances pushed to the `Compare` will
    /// be added to this new `Path`.
    pub fn split_path(&mut self) {
        self.paths.push(Path::new());
    }

    /// Split up the last [`Path`] in the [`Compare`] by starting
    /// a new [`Check`].
    pub fn split_check(&mut self, base: Base) {
        self.last_mut_must().push(Check::new(base));
    }

    /// Return a Region spanning the entirety of the [`Compare`], from
    /// the left [`Base`] in the first [`Path`] to the right (if it exists)
    /// `Base` in the last `Path`.
    pub fn get_region(&self) -> Region {
        let path_check = "path should have at least one check";
        let length_check = "length check should ensure safety";

        let first_path = self
            .paths
            .first()
            .expect("compare should have at least one path");

        let mut region = first_path.first().expect(path_check).left.get_region();
        if self.paths.len() > 1 {
            let check = self
                .paths
                .last()
                .expect(length_check)
                .last()
                .expect(path_check);

            let base = check.right.as_ref().unwrap_or(&check.left);
            region = region.combine(base.get_region());
        } else if first_path.len() > 1 {
            let last_check = first_path.last().expect(length_check);
            let base = last_check.right.as_ref().unwrap_or(&last_check.left);
            region = region.combine(base.get_region())
        }

        region
    }

    /// Return a mutable reference to the last [`Path`] in the [`Compare`].
    ///
    /// # Panics
    ///
    /// Will panic if no `Path` instances are in the `Compare`,
    /// which should never happen if created with .new().
    pub fn last_mut_must(&mut self) -> &mut Path {
        self.paths
            .last_mut()
            .expect("compare should always have >1 path")
    }

    /// Return a mutable reference to the last [`Check`] in the last [`Path`]
    /// in the [`Compare`].
    ///
    /// # Panics
    ///
    /// Will panic with the given reason if no `Check` is in the last
    /// `Path`.
    pub fn last_check_mut_must(&mut self, reason: &str) -> &mut Check {
        self.last_mut_must().last_mut().expect(reason)
    }
}

/// A single set of Check instances.
type Path = Vec<Check>;

/// Represents a comparison between two Base instances with some Operator.
///
/// If the Operator and second Base (right) are None, the first (left)
/// Base may be checked for a "truthy" value.
#[derive(Debug)]
pub struct Check {
    /// The Base to the left of the operator.
    pub left: Base,
    /// The operator used to compare left and right.
    pub operator: Option<Operator>,
    /// The Base to the right of the operator.
    pub right: Option<Base>,
}

impl Check {
    /// Create a new `Check` from the given `Base`.
    pub fn new(base: Base) -> Self {
        Self {
            left: base,
            operator: None,
            right: None,
        }
    }
}

/// Represents a call to render some kind of Expression.
#[derive(Debug, Clone)]
pub struct Output {
    pub expression: Expression,
    pub region: Region,
}

impl From<(Expression, Region)> for Output {
    /// Create an Output from the given (Expression, Region).
    fn from(value: (Expression, Region)) -> Self {
        Self {
            expression: value.0,
            region: value.1,
        }
    }
}

/// Variable types.
///
/// ## Literal
///
/// A literal value is some literal data, such as a string or number.
///
/// ## Variable
///
/// A variable is an Identifier such as "person.name" which indicates
/// the location of the true value within the Store.
#[derive(Debug, Clone, PartialEq)]
pub enum Base {
    /// A value located in the Store.
    Variable(Variable),
    /// A literal value located directly in the template source.
    Literal(Literal),
}

impl Base {
    /// Get a Region from the underlying Base kind.
    pub fn get_region(&self) -> Region {
        match self {
            Base::Variable(variable) => variable.get_region(),
            Base::Literal(literal) => literal.region,
        }
    }

    /// Return true when the `Base` is `Some(true)`.
    pub fn get_negate(&self) -> bool {
        match self {
            Base::Variable(variable) => variable.get_negate(),
            Base::Literal(literal) => literal.get_negate(),
        }
    }

    /// Set the `Base` negate property to the given value.
    pub fn set_negate(&mut self, value: bool) {
        match self {
            Base::Variable(variable) => variable.set_negate(value),
            Base::Literal(literal) => literal.set_negate(value),
        }
    }
}

/// Set of `Key` instances that can be used to locate data within the `Store`.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// A sequence of `Key` instances that indicates a path through the
    /// `Context` to some `Value`.
    pub path: Vec<Key>,
    /// True when the `Variable` should be negated.
    ///
    /// Option is used because it is possible that negation
    /// was not checked.
    pub negate: Option<bool>,
}

impl Variable {
    /// Create a new `Variable` from the given keys.
    pub fn new(path: Vec<Key>) -> Self {
        Self { path, negate: None }
    }

    /// Return true when the `Variable` is `Some(true)`.
    pub fn get_negate(&self) -> bool {
        self.negate.is_some_and(|n| n)
    }

    /// Set the negate property to the given value.
    pub fn set_negate(&mut self, value: bool) {
        self.negate = Some(value);
    }

    /// Get a Region spanning the area from the first and last Key instances,
    /// and including the negation character if negated.
    pub fn get_region(&self) -> Region {
        let mut region = self
            .path
            .first()
            .unwrap()
            .get_region()
            .combine(self.path.last().unwrap().get_region());

        if self.get_negate() {
            region.begin = region.begin - 1;
        }
        region
    }
}

/// Path segment in a larger identifier.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// TODO
    pub identifier: Identifier,
}

impl Key {
    /// Get a Region from the internal Identifier.
    pub fn get_region(&self) -> Region {
        self.identifier.region
    }
}

impl From<Identifier> for Key {
    /// Create a Key from the given Identifier.
    fn from(value: Identifier) -> Self {
        Self { identifier: value }
    }
}

/// Area that contains an identifying value.
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    /// TODO
    pub region: Region,
}

/// Literal data that does not need to be evaluated any further.
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    /// TODO
    pub value: Value,
    /// TODO
    pub region: Region,
    /// True when the `Literal` should be negated.
    ///
    /// Option is used because it is possible that negation
    /// was not checked.
    pub negate: Option<bool>,
}

impl Literal {
    /// Create a new `Literal` from the given `Value` and `Region`.
    pub fn new(value: Value, region: Region) -> Self {
        Self {
            value,
            region,
            negate: None,
        }
    }

    /// Return true when the `Literal` is `Some(true)`.
    pub fn get_negate(&self) -> bool {
        self.negate.is_some_and(|n| n)
    }

    /// Set the negate property to the given value.
    pub fn set_negate(&mut self, value: bool) {
        self.negate = Some(value);
    }

    /// Get a Region spanning the Literal text, including any quotes,
    /// and including the negation character if negated.
    pub fn get_region(&self) -> Region {
        let mut region = self.region;
        if self.negate.is_some_and(|n| n) {
            region.begin = region.begin - 1;
        }
        region
    }
}

/// Call to some registered function.
///
/// Refer to an underlying Expression from which the input data
/// may be derived.
#[derive(Debug, Clone)]
pub struct Call {
    /// TODO
    pub name: Identifier,
    /// TODO
    pub arguments: Option<Arguments>,
    /// TODO
    pub receiver: Box<Expression>,
    /// TODO
    pub region: Region,
}

/// Set of arguments that can be provided to a filter.
#[derive(Debug, Clone)]
pub struct Arguments {
    /// A list of arguments. The optional Region will be Some and point
    /// to the name of the argument, if it is not anonymous.
    pub values: Vec<(Option<Region>, Base)>,
    /// TODO
    pub region: Region,
}

/// Command to render another template.
#[derive(Debug, Clone)]
pub struct Include {
    /// TODO
    pub name: String,
    /// TODO
    pub globals: Option<Expression>,
}

/// Conditional rendering expression.
#[derive(Debug, Clone)]
pub struct IfElse {
    /// TODO
    pub not: bool,
    /// TODO
    pub condition: Expression,
    /// TODO
    pub then_branch: Scope,
    /// TODO
    pub else_branch: Option<Scope>,
}

/// Loop rendering expression.
#[derive(Debug, Clone)]
pub struct ForLoop {
    /// TODO
    pub not: bool,
    /// TODO
    pub condition: Expression,
    /// TODO
    pub then_branch: Scope,
    /// TODO
    pub else_branch: Option<Scope>,
}

/// Variable types derived from a loop.
pub enum LoopVariables {
    /// TODO
    Item(Identifier),
    /// TODO
    KeyValue(KeyValue),
}

/// Key/value pair.
pub struct KeyValue {
    /// TODO
    pub key: Identifier,
    /// TODO
    pub value: Identifier,
    /// TODO
    pub region: Region,
}
