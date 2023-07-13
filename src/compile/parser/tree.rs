use crate::{compile::parser::scope::Scope, region::Region};
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
    /// A Simple call to render the named value from the context.
    Base(Base),
    /// A more complex variant which typically retrieves some value from the context and modifies
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
/// the location of the true value within the context.
#[derive(Debug, Clone, PartialEq)]
pub enum Base {
    Variable(Variable),
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

/// Set of Key instances that can be used to locate data within the context.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
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
    pub region: Region,
}

/// Literal data that does not need to be evaluated any further.
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub value: Value,
    pub region: Region,
}

/// Call to some registered function.
///
/// Refer to an underlying Expression from which the input data
/// may be derived.
#[derive(Debug, Clone)]
pub struct Call {
    pub name: Identifier,
    pub arguments: Option<Arguments>,
    pub receiver: Box<Expression>,
    pub region: Region,
}

/// Set of arguments that can be provided to a filter.
#[derive(Debug, Clone)]
pub struct Arguments {
    pub values: Vec<(Option<Region>, Base)>,
    pub region: Region,
}

/// Command to render another template.
#[derive(Debug, Clone)]
pub struct Include {
    pub name: String,
    pub globals: Option<Expression>,
}

/// Conditional rendering expression.
#[derive(Debug, Clone)]
pub struct IfElse {
    pub not: bool,
    pub condition: Expression,
    pub then_branch: Scope,
    pub else_branch: Option<Scope>,
}

/// Loop rendering expression.
#[derive(Debug, Clone)]
pub struct ForLoop {
    pub not: bool,
    pub condition: Expression,
    pub then_branch: Scope,
    pub else_branch: Option<Scope>,
}

/// Variable types derived from a loop.
pub enum LoopVariables {
    Item(Identifier),
    KeyValue(KeyValue),
}

/// Key/value pair.
pub struct KeyValue {
    pub key: Identifier,
    pub value: Identifier,
    pub region: Region,
}
