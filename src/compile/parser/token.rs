use crate::types::Region;

#[derive(Copy, Clone)]
pub enum Token {
    Raw,
    If(If),
    Expression(Expression),
    Let(Let),
    For(For),
}

#[derive(Copy, Clone)]
pub struct If {}

#[derive(Copy, Clone)]
pub struct Expression {
    // We know this will be embedded inside of a region, but that region
    // will span the entire expression which eventually will include the filters, etc..
    //
    // So how do we know what the ident value is?
    ident: Region, // Just take the &str from the lexer::Token?
}

#[derive(Copy, Clone)]
pub struct Let {}

#[derive(Copy, Clone)]
pub struct For {}
