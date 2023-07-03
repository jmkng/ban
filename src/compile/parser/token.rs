#[derive(Copy, Clone)]
pub enum Token {
    Raw,
    If(If),
    Let(Let),
    For(For),
}

#[derive(Copy, Clone)]
pub struct If {}

#[derive(Copy, Clone)]
pub struct Let {}

#[derive(Copy, Clone)]
pub struct For {}
