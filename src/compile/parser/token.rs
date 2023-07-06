use crate::{compile::parser::scope::Scope, region::Region};
use serde_json::Value;

// Stmt
pub enum Token {
    Raw(Region),
    InlineExpression(InlineExpression),
    Include(Include),
    IfElse(IfElse),
    ForLoop(ForLoop),
}

pub struct InlineExpression {
    pub expression: Expression,
    pub region: Region,
}

pub enum Expression {
    BaseExpression(BaseExpression),
    Call(Call),
}

pub enum BaseExpression {
    Var(Var),
    Literal(Literal),
}

pub struct Var {
    pub path: Vec<Key>,
}

pub enum Key {
    List(Index),
    Map(Identifier),
}

pub struct Index {
    pub value: usize,
    pub region: Region,
}

pub struct Identifier {
    pub region: Region,
}

pub struct Literal {
    pub value: Value,
    pub region: Region,
}

pub struct Call {
    pub name: Identifier,
    pub args: Option<Arguments>,
    pub receiver: Box<Expression>,
    pub region: Region,
}

pub struct Arguments {
    pub values: Vec<BaseExpression>,
    pub region: Region,
}

pub struct Include {
    pub name: String,
    pub globals: Option<Expression>,
}

pub struct IfElse {
    pub not: bool,
    pub condition: Expression,
    pub then_branch: Scope,
    pub else_branch: Option<Scope>,
}

pub struct ForLoop {
    pub not: bool,
    pub condition: Expression,
    pub then_branch: Scope,
    pub else_branch: Option<Scope>,
}

pub enum LoopVariables {
    Item(Identifier),
    KeyValue(KeyValue),
}

pub struct KeyValue {
    pub key: Identifier,
    pub value: Identifier,
    pub region: Region,
}
