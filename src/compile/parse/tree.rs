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

/// Represents a comparison between two Expressions with some Operator.
///
/// If the Operator and second Expression (right) are None, the first (left)
/// Expression may be checked for a "truthy" value.
pub struct Comparison {
    /// The Expression to the left of the operator.
    /// Boolean indicates negation.
    pub left: (bool, Expression),
    /// The operator used to compare left and right.
    pub operator: Option<Operator>,
    /// The Expression to the right of the operator.
    /// Boolean indicates negation.
    pub right: Option<(bool, Expression)>,
    /// Location of the Comparison.
    pub region: Region,
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
}

/// Set of Key instances that can be used to locate data within the Store.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// TODO
    pub path: Vec<Key>,
}

impl Variable {
    /// Get a Region spanning the area from the first and last Key instances.
    pub fn get_region(&self) -> Region {
        self.path
            .first()
            .unwrap()
            .get_region()
            .combine(self.path.last().unwrap().get_region())
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
    /// TODO
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
