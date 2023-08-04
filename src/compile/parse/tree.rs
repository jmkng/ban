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
    For(For),
    /// Variable assignment.
    Let(Let),
    /// Template block.
    Block(Block),
}

/// Represents a section of text that may be overridden by another [`Block`].
#[derive(Debug, Clone)]
pub struct Block {
    /// The name of the [`Block`].
    pub name: Base,
    /// The data inside of the [`Block`].
    pub scope: Scope,
    /// The location of the [`Block`].
    pub region: Region,
}

/// A call to extend another named [`Template`][`crate::Template`] by overriding
/// its [`Block`] instances
#[derive(Debug, Clone)]
pub struct Extends {
    /// The name of the template to extend.
    pub name: Base,
    /// The location of the [`Extends`].
    pub region: Region,
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

/// Storage for a stack of [`CheckPath`] instances.
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

/// Set of [`Identifier`] instances that can be used to locate data
/// within the [`Store`][`crate::Store`].
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// A set of [`Identifier`] instances that form  a path through the
    /// [`Store`][`crate::Store`] to some [`Value`].
    pub path: Vec<Identifier>,
}

impl Variable {
    /// Create a new [`Variable`] from the given keys.
    pub fn new(path: Vec<Identifier>) -> Self {
        Self { path }
    }

    /// Get a [`Region`] from the first to last [`Identifier`] instance.
    pub fn get_region(&self) -> Region {
        self.path
            .first()
            .unwrap()
            .region
            .combine(self.path.last().unwrap().region)
    }
}

/// Area that contains an identifying value.
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    /// The location of the [`Identifier`].
    pub region: Region,
}

/// Literal data that does not need to be evaluated any further.
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    /// A [`Value`] representing the [`Literal`].
    pub value: Value,
    /// The location of the [`Literal`].
    pub region: Region,
}

impl Literal {
    /// Create a new `Literal` from the given `Value` and `Region`.
    pub fn new(value: Value, region: Region) -> Self {
        Self { value, region }
    }
}

/// Command to execute a [`Filter`][`crate::filter::Filter`].
#[derive(Debug, Clone)]
pub struct Call {
    /// The name of the [`Filter`][`crate::filter::Filter`].
    pub name: Identifier,
    /// [`Arguments`] passed to the [`Filter`][`crate::filter::Filter`].
    pub arguments: Option<Arguments>,
    /// The source of the input data.
    pub receiver: Box<Expression>,
    /// The location of the [`Call`].
    ///
    /// This `Region` points to the `Call` itself, and all calls "above" it.
    pub region: Region,
}

/// Set of arguments that can be provided to a
/// [`Filter`][`crate::filter::Filter`].
#[derive(Debug, Clone)]
pub struct Arguments {
    /// A set of [`Argument`] instances, representing the arguments
    /// passed to a [`Filter`][`crate::filter::Filter`].
    pub values: Vec<Argument>,
    /// The location of the [`Arguments`].
    pub region: Region,
}

/// A single argument.
#[derive(Debug, Clone)]
pub struct Argument {
    /// The name of the [`Argument`], may be None
    pub name: Option<Region>,
    /// The value of the [`Argument`].
    pub value: Base,
}

impl Argument {
    /// Get a [`Region`] from the name, if it exists, to the value.
    pub fn get_region(&self) -> Region {
        if self.name.is_some() {
            self.name.unwrap().combine(self.value.get_region())
        } else {
            self.value.get_region()
        }
    }
}

/// Set of arguments associated with an [`Include`].
#[derive(Debug, Clone)]
pub struct Mount {
    /// A set of [`Point`] instances, representing the arguments
    /// passed to an [`Include`].
    pub values: Vec<Point>,
    /// The location of the [`Mount`].
    pub region: Region,
}

/// A name associated with a [`Base`].
///
/// Similar to [`Argument`], but requires a name.
#[derive(Debug, Clone)]
pub struct Point {
    /// The name of the [`Point`].
    pub name: Region,
    /// The value of the [`Point`].
    pub value: Base,
}

/// Command to render another template.
#[derive(Debug, Clone)]
pub struct Include {
    /// The name of the [`Template`][`crate::Template`] to render.
    pub name: Base,
    /// An optional set of scoped values to render the [`Include`]
    /// with.
    pub mount: Option<Mount>,
}

/// Conditional rendering block.
#[derive(Debug, Clone)]
pub struct If {
    /// Contains the data needed to determine which branch to render.
    pub tree: CheckTree,
    /// The [`Scope`] to render if the [`CheckTree`] contains a truthy
    /// [`CheckBranch`].
    pub then_branch: Scope,
    /// The [`Scope`] to render if the [`CheckTree`] does not contain
    /// a truthy [`CheckBranch`].
    pub else_branch: Option<Scope>,
    /// The location of the [`If`].
    pub region: Region,
}

/// Loop rendering block.
#[derive(Debug, Clone)]
pub struct For {
    /// Faux variables that contain the assignment for each iteration.
    pub set: Set,
    /// The [`Base`] to be iterated on.
    pub base: Base,
    /// The [`Scope`] that is rendered with each iteration.
    pub scope: Scope,
    /// The location of the [`Iterable`].
    pub region: Region,
}

/// Variable types derived from a loop.
#[derive(Debug, Clone)]
pub enum Set {
    /// A single variable, such as `(* for this in that *)`.
    Single(Identifier),
    /// A pair of variables, such as `(* for i, this in that *)`.
    Pair(KeyValue),
}

/// Key/value pair.
#[derive(Debug, Clone)]
pub struct KeyValue {
    /// Contains the "primary" value in a for loop.
    pub value: Identifier,
    /// Contains the "secondary" value in a for loop,
    /// such as an index or key.
    pub key: Identifier,
    /// The location of the [`KeyValue`].
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
