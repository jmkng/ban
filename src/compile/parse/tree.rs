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
    /// An if or else if block.
    If(If),
    /// A for loop.
    For(Iterable),
    /// Variable assignment.
    Let(Let),
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
    /// Get the Region from the underlying [`Expression`] kind.
    pub fn get_region(&self) -> Region {
        match self {
            Expression::Base(base) => base.get_region(),
            Expression::Call(call) => call.region,
        }
    }
}

/// Storage for a "stack" of [`CheckPath`] instances.
///
/// For example, this example represents one [`CheckTree`] made up of two
/// [`CheckPath`] instances, which are separated at the "||" characters:
///
/// this > that && those == these || they > them
#[derive(Debug, Clone)]
pub struct CheckTree {
    pub branches: Vec<CheckBranch>,
}

impl CheckTree {
    /// Create a new [`CheckTree`] with one initialized [`CheckBranch`].
    pub fn new() -> Self {
        Self {
            branches: vec![CheckBranch::new()],
        }
    }

    /// Split up the [`CheckTree`] by adding a new [`CheckBranch`].
    ///
    /// Any additional [`Check`] instances pushed to the `CheckTree` will
    /// be added to this new `CheckBranch`.
    pub fn split_branch(&mut self) {
        self.branches.push(CheckBranch::new());
    }

    /// Split up the last [`CheckBranch`] in the [`CheckTree`] by starting
    /// a new [`Check`].
    pub fn split_check(&mut self, check: Check) {
        self.last_mut_must().push(check);
    }

    /// Return a Region spanning the entirety of the [`CheckTree`], from
    /// the left [`Base`] in the first [`CheckBranch`] to the right
    /// (if it exists) `Base` in the last `CheckBranch`.
    pub fn get_region(&self) -> Region {
        let branch_check = "branch should have at least one check";
        let length_check = "length check should ensure safety";

        let first_branch = self
            .branches
            .first()
            .expect("tree should have at least one branch");

        let mut region = first_branch.first().expect(branch_check).left.get_region();
        if self.branches.len() > 1 {
            let check = self
                .branches
                .last()
                .expect(length_check)
                .last()
                .expect(branch_check);

            let base = check.right.as_ref().unwrap_or(&check.left);
            region = region.combine(base.get_region());
        } else if first_branch.len() > 1 {
            let last_check = first_branch.last().expect(length_check);
            let base = last_check.right.as_ref().unwrap_or(&last_check.left);
            region = region.combine(base.get_region())
        }

        region
    }

    /// Return a mutable reference to the last [`CheckBranch`] in the
    /// [`CheckTree`].
    ///
    /// # Panics
    ///
    /// Will panic if no `CheckBranch` instances are in the `CheckTree`,
    /// which should never happen if created with .new().
    pub fn last_mut_must(&mut self) -> &mut CheckBranch {
        self.branches
            .last_mut()
            .expect("tree should always have >1 branch")
    }

    /// Return a mutable reference to the last [`Check`] in the last
    /// [`CheckBranch`] in the [`CheckTree`].
    ///
    /// # Panics
    ///
    /// Will panic with the given reason if no `Check` is in the last
    /// `CheckBranch`.
    pub fn last_check_mut_must(&mut self, reason: &str) -> &mut Check {
        self.last_mut_must().last_mut().expect(reason)
    }
}

/// A single set of [`Check`] instances.
pub type CheckBranch = Vec<Check>;

/// Represents a comparison between two [`Base`] instances with some
/// [`Operator`].
#[derive(Debug, Clone)]
pub struct Check {
    /// True if the [`Check`] is negated.
    pub negate: bool,
    /// The [`Base`] to the left of the [`Operator`].
    pub left: Base,
    /// The [`Operator`] used to compare left and right.
    pub operator: Option<Operator>,
    /// The [`Base`] to the right of the [`Operator`].
    pub right: Option<Base>,
}

impl Check {
    /// Create a new [`Check`] from the given [`Base`].
    pub fn new(base: Base, negate: bool) -> Self {
        Self {
            negate,
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
    /// Create an Output from the given ([`Expression`], [`Region`]).
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
/// A [`Literal`] is some literal data that does not need to be searched
/// for in the `Store` at render time, such as a string or number.
///
/// ## Variable
///
/// A [`Variable`] is an Identifier such as "person.name" which represents
/// a path to a [`Value`] within the `Store`.
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
}

/// Set of `Key` instances that can be used to locate data within the `Store`.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// A sequence of `Key` instances that indicates a path through the
    /// `Context` to some `Value`.
    pub path: Vec<Key>,
}

impl Variable {
    /// Create a new `Variable` from the given keys.
    pub fn new(path: Vec<Key>) -> Self {
        Self { path }
    }

    /// Get a Region spanning the area from the first and last Key instances.
    pub fn get_region(&self) -> Region {
        let mut region = self
            .path
            .first()
            .unwrap()
            .get_region()
            .combine(self.path.last().unwrap().get_region());
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
}

impl Literal {
    /// Create a new `Literal` from the given `Value` and `Region`.
    pub fn new(value: Value, region: Region) -> Self {
        Self { value, region }
    }
}

/// Call to some registered function.
///
/// Refer to an underlying [`Expression`] from which the input data
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

/// Conditional rendering block.
#[derive(Debug, Clone)]
pub struct If {
    /// TODO
    pub tree: CheckTree,
    /// TODO
    pub then_branch: Scope,
    /// TODO
    pub else_branch: Option<Scope>,
    /// TODO
    pub region: Region,
}

/// Loop rendering block.
#[derive(Debug, Clone)]
pub struct Iterable {
    /// TODO
    pub set: Set,
    /// TODO
    pub base: Base,
    /// TODO
    pub data: Scope,
    /// TODO
    pub region: Region,
}

/// Variable types derived from a loop.
#[derive(Debug, Clone)]
pub enum Set {
    /// TODO
    Single(Identifier),
    /// TODO
    Pair(KeyValue),
}

/// Key/value pair.
#[derive(Debug, Clone)]
pub struct KeyValue {
    /// TODO
    pub key: Identifier,
    /// TODO
    pub value: Identifier,
    /// TODO
    pub region: Region,
}

/// Assignment of left to right.
#[derive(Debug, Clone)]
pub struct Let {
    /// The variable name.
    pub left: Identifier,
    /// The value to be assigned to the variable name.
    pub right: Base,
}
