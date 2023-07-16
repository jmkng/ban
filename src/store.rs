use crate::Error;
use serde::Serialize;
use serde_json::{to_value, Value};
use std::{collections::HashMap, fmt::Display};

/// Provides storage for data that templates can be rendered against.
pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    /// Create a new Store.
    #[inline]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert the value into the Store.
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    pub fn insert<S, T>(&mut self, key: S, value: T) -> Result<(), Error>
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        let serialized = to_value(&value)
            .map_err(|_| Err(Error::build(format!("value {} is unserializable", value))));

        match serialized {
            Ok(value) => {
                self.data.insert(key.into(), value);
                Ok(())
            }
            Err(err) => err,
        }
    }

    /// Insert the value into the Store.
    ///
    /// # Panics
    ///
    /// Will panic if the serialization fails.
    #[inline]
    pub fn insert_must<S, T>(&mut self, key: S, value: T)
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.data.insert(key.into(), to_value(value).unwrap());
    }

    /// Insert the value into the Store.
    ///
    /// Returns the Store, so additional methods may be chained.
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    pub fn with<S, T>(mut self, key: S, value: T) -> Result<Self, Error>
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.insert(key, value)?;
        Ok(self)
    }

    /// Insert the value into the Store.
    ///
    /// Returns the Store, so additional methods may be chained.
    ///
    /// # Panics
    ///
    /// Will panic if the serialization fails.
    #[inline]
    pub fn with_must<S, T>(mut self, key: S, value: T) -> Self
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.insert_must(key, value);
        self
    }

    /// Get the value of the given key, if any.
    #[inline]
    pub fn get(&self, index: &str) -> Option<&Value> {
        self.data.get(index)
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;

    #[test]
    fn test_insert() {
        let mut store = Store::new();
        store.insert_must("one", "two");

        assert!(store
            .get("one")
            .is_some_and(|t| t.as_str().unwrap() == "two"));
    }

    #[test]
    fn test_insert_fluent() {
        assert!(Store::new()
            .with_must("three", "four")
            .get("three")
            .is_some_and(|t| t.as_str().unwrap() == "four"))
    }
}
