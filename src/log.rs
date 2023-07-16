mod error;
mod visual;

pub use error::*;
pub use visual::{Pointer, Visual};

const RED: &str = "\x1B[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1B[0m";
