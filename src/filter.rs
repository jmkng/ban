//! Playground for template filter implementation
use crate::Error;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Trait which all filter functions must implement.
///
/// The input parameter refers to the value that is being operated on,
/// while args may contain any additional values passed in to the function.
///
/// # Examples
///
/// Given this expression:
///
/// ```html
/// (( name | prepend 1: "hello, " | append "!" | upper ))
/// ```
///
/// The "name" value is located within the context and provided to the first
/// filter function, "prepend", as input. One argument called "1" is assigned
/// to the string literal "hello, ", and is named "1".
///
/// The second filter, "append", receives the output of the first function as
/// input. It is provided one string literal as an argument as well, however,
/// it is not given a name.
///
/// Anonymous arguments are automatically provided a name which is equal to 1
/// plus the amount of anonymous arguments that have been previously assigned,
/// so in this case it would also be called "1" and is accessible like so:
///
/// ```rs
/// args.get("1")
/// ```
pub trait Filter {
    fn apply(&self, input: &Value, args: &HashMap<&str, Value>) -> Result<Value, Error>;
}

/// Example filter for testing.
///
/// Return any i64 or f64 multipled by 2.
struct DoubleFilter;

impl Filter for DoubleFilter {
    fn apply(&self, input: &Value, _args: &HashMap<&str, Value>) -> Result<Value, Error> {
        match input {
            Value::Number(number) => {
                if let Some(number) = number.as_i64() {
                    Ok(json!(number * 2))
                } else if let Some(number) = number.as_f64() {
                    Ok(json!(number * 2.0))
                } else {
                    Err(Error::General("invalid numeric value".into()))
                }
            }
            _ => Err(Error::General("expected numeric value".into())),
        }
    }
}

#[test]
fn test_filter() {
    let mut filters: HashMap<&str, Box<dyn Filter>> = HashMap::new();
    filters.insert("double", Box::new(DoubleFilter));

    let filter = filters.get("double").unwrap();
    assert_eq!(filter.apply(&json!(42), &HashMap::new()), Ok(json!(84)));
    assert_eq!(filter.apply(&json!(42.0), &HashMap::new()), Ok(json!(84.0)));
}
