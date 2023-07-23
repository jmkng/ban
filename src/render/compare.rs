use crate::{compile::Operator, filter::Error, log::INCOMPATIBLE_TYPES};
use serde_json::{json, Value};

/// Return true if the given [`Value`] is truthy.
pub fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(bo) => *bo,
        Value::Number(nu) => nu.as_f64().unwrap() > 0.0f64,
        Value::String(st) => !st.is_empty(),
        Value::Array(ar) => !ar.is_empty(),
        Value::Object(ob) => !ob.is_empty(),
        Value::Null => false,
    }
}

/// Compare the two [`Value`] instances with the given [`Operator`].
///
/// # Errors
///
/// Returns an [`Error`] if the two types cannot be compared, or the `Operator`
/// cannot be applied to the types.
pub fn compare_values(left: &Value, operator: Operator, right: &Value) -> Result<bool, Error> {
    let result = match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            let left_as = left.as_f64().unwrap();
            let right_as = right.as_f64().unwrap();
            match operator {
                Operator::Add => is_truthy(&json!(left_as + right_as)),
                Operator::Subtract => is_truthy(&json!(left_as - right_as)),
                Operator::Multiply => is_truthy(&json!(left_as * right_as)),
                Operator::Divide => is_truthy(&json!(left_as / right_as)),
                Operator::Greater => left_as > right_as,
                Operator::Lesser => left_as < right_as,
                Operator::Equal => left_as == right_as,
                Operator::NotEqual => left_as != right_as,
                Operator::GreaterOrEqual => left_as >= right_as,
                Operator::LesserOrEqual => left_as <= right_as,
            }
        }
        (Value::String(left), Value::String(right)) => match operator {
            Operator::Add => is_truthy(&Value::String(format!("{left}{right}"))),
            Operator::Greater => left > right,
            Operator::Lesser => left < right,
            Operator::Equal => left == right,
            Operator::NotEqual => left != right,
            Operator::GreaterOrEqual => left >= right,
            Operator::LesserOrEqual => left <= right,
            unsupported => {
                return Err(Error::build(INCOMPATIBLE_TYPES).help(format!(
                    "operator `{unsupported}` is invalid on string types"
                )))
            }
        },
        (Value::Bool(left), Value::Bool(right)) => match operator {
            Operator::Greater => left > right,
            Operator::Lesser => left < right,
            Operator::Equal => left == right,
            Operator::NotEqual => left != right,
            Operator::GreaterOrEqual => left >= right,
            Operator::LesserOrEqual => left <= right,
            unsupported => {
                return Err(Error::build(INCOMPATIBLE_TYPES).help(format!(
                    "operator `{unsupported}` is invalid on boolean types"
                )))
            }
        },
        (Value::Array(left), Value::Array(right)) => match operator {
            Operator::Add => is_truthy(&json!(left.len() + right.len())),
            Operator::Subtract => is_truthy(&json!(left.len() - right.len())),
            Operator::Multiply => is_truthy(&json!(left.len() * right.len())),
            Operator::Divide => is_truthy(&json!(left.len() / right.len())),
            Operator::Greater => left.len() > right.len(),
            Operator::Lesser => left.len() < right.len(),
            Operator::Equal => left.len() == right.len(),
            Operator::NotEqual => left.len() != right.len(),
            Operator::GreaterOrEqual => left.len() >= right.len(),
            Operator::LesserOrEqual => left.len() <= right.len(),
        },
        (Value::Object(left), Value::Object(right)) => match operator {
            Operator::Add => is_truthy(&json!(left.len() + right.len())),
            Operator::Subtract => is_truthy(&json!(left.len() - right.len())),
            Operator::Multiply => is_truthy(&json!(left.len() * right.len())),
            Operator::Divide => is_truthy(&json!(left.len() / right.len())),
            Operator::Greater => left.len() > right.len(),
            Operator::Lesser => left.len() < right.len(),
            Operator::Equal => left.len() == right.len(),
            Operator::NotEqual => left.len() != right.len(),
            Operator::GreaterOrEqual => left.len() >= right.len(),
            Operator::LesserOrEqual => left.len() <= right.len(),
        },
        (left, right) => {
            return Err(Error::build(INCOMPATIBLE_TYPES).help(format!(
                "types `{}` and `{}` cannot be compared",
                left, right
            )))
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{compile, compile::Operator, render, Store};
    use serde_json::{json, Value};

    #[test]
    fn test_truthy_boolean() {
        let template = compile("(* if value *)a(* else *)b(* endif *)").unwrap();
        let true_values = vec![
            json!("lorem"),
            json!(12),
            json!(114.4),
            json!(true),
            json!(vec!["lorem", "ipsum"]),
            json!({"lorem": "ipsum"}),
        ];
        let false_values = vec![
            json!(""),
            json!(0),
            json!(0.0),
            json!(-12),
            json!(false),
            json!(vec![""; 0]),
            json!({}),
        ];
        let mut store = Store::new();
        for (left, right) in true_values.into_iter().zip(false_values) {
            store.insert_must("value", left);
            // println!("{:?}", store.get("value"));
            assert_eq!(render(&template, &store).unwrap(), "a");
            store.insert_must("value", right);
            // println!("{:?}", store.get("value"));
            assert_eq!(render(&template, &store).unwrap(), "b");
        }
    }

    #[test]
    fn incompatible_types() {
        let template = compile("(* if \"hello\" > true *)a(* endif *)");
        assert!(template.is_ok());
        let result = render(&template.unwrap(), &Store::new());
        assert!(result.is_err());

        // println!("{:#}", result.unwrap_err());

        // error: incompatible types
        // --> ?:1:7
        // |
        // 1 | (* if "hello" > true *)a(* endif *)
        // |         ^^^^^^^^^^^^^^
        // |
        // = help: types `"hello"` and `true` cannot be compared
    }

    #[test]
    fn incompatible_operator() {
        let template = compile("(* if true + false *)a(* endif *)");
        assert!(template.is_ok());
        let result = render(&template.unwrap(), &Store::new());
        assert!(result.is_err());

        // println!("{:#}", result.unwrap_err());

        // error: incompatible types
        // --> ?:1:7
        // |
        // 1 | (* if true + false *)a(* endif *)
        // |         ^^^^^^^^^^^^
        // |
        // = help: operator `+` is invalid on boolean types
    }

    #[test]
    fn test_truthy_add() {
        let left = vec![json!(0), json!(""), json!(["one", "two"]), json!({})];
        let right = vec![json!(1), json!("hi"), json!(["one"]), json!({"a": "b"})];
        test_truthy_compare(left, right, Operator::Add);
    }

    #[test]
    fn test_truthy_subtract() {
        let left = vec![
            json!(20),
            json!(["one", "two"]),
            json!({"a": "b", "c": "d"}),
        ];
        let right = vec![json!(10), json!(["one"]), json!({"a": "b"})];
        test_truthy_compare(left, right, Operator::Subtract);
    }

    #[test]
    fn test_truthy_multiply() {
        let left = vec![
            json!(10),
            json!(["one", "two"]),
            json!({"a": "b", "c": "d"}),
        ];
        let right = vec![json!(10), json!(["one"]), json!({"a": "b"})];
        test_truthy_compare(left, right, Operator::Multiply);
    }

    #[test]
    fn test_truthy_divide() {
        let left = vec![
            json!(100),
            json!(["one", "two"]),
            json!({"a": "b", "c": "d"}),
        ];
        let right = vec![json!(10), json!(["one"]), json!({"a": "b"})];
        test_truthy_compare(left, right, Operator::Divide);
    }

    #[test]
    fn test_truthy_greater() {
        let left = vec![json!(100), json!("b"), json!(true)];
        let right = vec![json!(50), json!("a"), json!(false)];
        test_truthy_compare(left, right, Operator::Greater);
    }

    #[test]
    fn test_truthy_lesser() {
        let left = vec![json!(50), json!("a"), json!(false)];
        let right = vec![json!(5100), json!("b"), json!(true)];
        test_truthy_compare(left, right, Operator::Lesser);
    }

    #[test]
    fn test_truthy_equal() {
        let left = vec![
            json!(10),
            json!("a"),
            json!(true),
            json!(["one"]),
            json!({"a": "b"}),
        ];
        let right = vec![
            json!(10),
            json!("a"),
            json!(true),
            json!(["one"]),
            json!({"a": "b"}),
        ];
        test_truthy_compare(left, right, Operator::Equal);
    }

    #[test]
    fn test_truthy_not_equal() {
        let left = vec![
            json!(10),
            json!("a"),
            json!(true),
            json!(["one"]),
            json!({"a": "b"}),
        ];
        let right = vec![
            json!(20),
            json!("b"),
            json!(false),
            json!(["one", "two"]),
            json!({"a": "b", "c": "d"}),
        ];
        test_truthy_compare(left, right, Operator::NotEqual);
    }

    #[test]
    fn test_truthy_greater_equal() {
        let left = vec![
            json!(10),
            json!(11),
            json!("a"),
            json!("b"),
            json!(false),
            json!(true),
            json!(["one"]),
            json!(["one", "two"]),
            json!({"a": "b"}),
            json!({"a": "b", "c": "d"}),
        ];
        let right = vec![
            json!(10),
            json!(10),
            json!("a"),
            json!("a"),
            json!(false),
            json!(false),
            json!(["one"]),
            json!(["one"]),
            json!({"a": "b"}),
            json!({"a": "b"}),
        ];
        test_truthy_compare(left, right, Operator::GreaterOrEqual);
    }

    #[test]
    fn test_truthy_lesser_equal() {
        let left = vec![
            json!(10),
            json!(10),
            json!("a"),
            json!("a"),
            json!(false),
            json!(false),
            json!(["one"]),
            json!(["one"]),
            json!({"a": "b"}),
            json!({"a": "b"}),
        ];
        let right = vec![
            json!(10),
            json!(11),
            json!("a"),
            json!("b"),
            json!(false),
            json!(true),
            json!(["one"]),
            json!(["one", "two"]),
            json!({"a": "b"}),
            json!({"a": "b", "c": "d"}),
        ];
        test_truthy_compare(left, right, Operator::LesserOrEqual);
    }

    // Zip the two Vec<Value> instances together and compare them in a template with the
    // given `Operator`.
    fn test_truthy_compare(left: Vec<Value>, right: Vec<Value>, operator: Operator) {
        let source = format!("(* if left {} right *)a(* endif *)", operator);
        let template = compile(&source).unwrap();

        let mut store = Store::new();
        for (left, right) in left.into_iter().zip(right) {
            store.insert_must("left", left);
            store.insert_must("right", right);
            // println!("{:?} | {:?}", store.get("left"), store.get("right"));
            let result = render(&template, &store).unwrap();
            assert_eq!(result, "a");
        }
    }
}
