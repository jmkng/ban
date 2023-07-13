use std::collections::HashMap;

use serde::Serialize;
use serde_json::{to_value, Value};

use crate::{error::Error, general_error};

/// Provides storage for data that templates can be rendered against.
pub struct Context {
    data: HashMap<String, Value>,
}

impl Context {
    /// Create a new Context.
    #[inline]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert the value into the Context.
    ///
    /// # Panics
    ///
    /// Will panic if the serialization fails.
    #[inline]
    pub fn insert_must<S, T>(&mut self, key: S, value: T)
    where
        S: Into<String>,
        T: Serialize,
    {
        self.data.insert(key.into(), to_value(value).unwrap());
    }

    /// Insert the value into the Context.
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    pub fn insert<S, T>(&mut self, key: S, value: T) -> Result<(), Error>
    where
        S: Into<String>,
        T: Serialize,
    {
        let serialized = to_value(&value).map_err(|_| general_error!("unable to serialize value"));
        match serialized {
            Ok(value) => {
                self.data.insert(key.into(), value);
                Ok(())
            }
            Err(err) => err,
        }
    }

    /// Get the value of the given key, if any.
    #[inline]
    pub fn get(&self, index: &str) -> Option<&Value> {
        self.data.get(index)
    }
}
