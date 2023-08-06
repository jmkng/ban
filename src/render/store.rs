use std::{collections::HashMap, fmt::Display};

use crate::log::Error;

use serde::Serialize;
use serde_json::{to_value, Value};

/// Provides storage for data that a [`Template`][`crate::Template`] can be
/// rendered with.
#[derive(Debug)]
pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    /// Create a new [`Store`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let store = Store::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Inserts a key-value pair into the [`Store`].
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let mut store = Store::new();
    /// let result = store.insert("name", "taylor");
    ///
    /// assert!(result.is_ok());
    /// ```
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

    /// Inserts a key-value pair into the [`Store`].
    ///
    /// # Panics
    ///
    /// Panics if the serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let mut store = Store::new();
    /// store.insert_must("name", "taylor");
    /// ```
    #[inline]
    pub fn insert_must<S, T>(&mut self, key: S, value: T)
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.data.insert(key.into(), to_value(value).unwrap());
    }

    /// Inserts a key-value pair into the [`Store`].
    ///
    /// Returns the `Store`, so additional methods may be chained.
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let mut store = Store::new().with("name", "taylor");
    ///
    /// assert!(store.is_ok());
    /// ```
    #[inline]
    pub fn with<S, T>(mut self, key: S, value: T) -> Result<Self, Error>
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.insert(key, value)?;

        Ok(self)
    }

    /// Inserts a key-value pair into the [`Store`].
    ///
    /// Returns the `Store`, so additional methods may be chained.
    ///
    /// # Panics
    ///
    /// Panics if the serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let mut store = Store::new().with_must("name", "taylor");
    /// ```
    #[inline]
    pub fn with_must<S, T>(mut self, key: S, value: T) -> Self
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.insert_must(key, value);

        self
    }

    /// Returns a reference to the [`Value`] corresponding to the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use ban::Store;
    ///
    /// let store = Store::new().with_must("name", "taylor");
    /// let result = store.get("name");
    ///
    /// assert_eq!(result.unwrap(), "taylor")
    /// ```
    #[inline]
    pub fn get(&self, index: &str) -> Option<&Value> {
        self.data.get(index)
    }
}

// Wrapper for [`Store`] that provides mutable storage for shadowed values.
#[derive(Debug)]
pub struct Shadow<'store> {
    pub store: &'store Store,
    data: Vec<HashMap<String, Value>>,
}

impl<'store> Shadow<'store> {
    /// Create a new [`Shadow`] over the given [`Store`].
    #[inline]
    pub fn new(store: &'store Store) -> Self {
        Self {
            store,
            data: vec![HashMap::new()],
        }
    }

    /// Push a new frame onto the [`Shadow`].
    #[inline]
    pub fn push(&mut self) {
        self.data.push(HashMap::new());
    }

    /// Remove the top frame from the [`Shadow`].
    #[inline]
    pub fn pop(&mut self) {
        if self.data.len() == 1 {
            panic!("last scope must never be removed");
        }
        self.data.pop();
    }

    /// Insert the value into the top level stack of the [`Shadow`].
    ///
    /// # Panics
    ///
    /// Panics if no frames exist within the [`Shadow`].
    #[inline]
    pub fn insert_must<S, T>(&mut self, key: S, value: T)
    where
        S: Into<String>,
        T: Serialize + Display,
    {
        self.data
            .last_mut()
            .expect("stack must not be empty when shadowing value")
            .insert(key.into(), to_value(value).unwrap());
    }

    /// Get the [`Value`] of the given key.
    ///
    /// If the key is not found within the [`Shadow`], the store will be
    /// searched.
    #[inline]
    pub(crate) fn get(&self, index: &str) -> Option<&Value> {
        for stack in self.data.iter().rev() {
            if let Some(value) = stack.get(index) {
                return Some(value);
            }
        }
        self.store.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::Shadow;
    use crate::Store;
    use serde_json::json;

    #[test]
    fn test_store_insert() {
        let mut store = Store::new();
        store.insert_must("one", "two");

        assert!(store
            .get("one")
            .is_some_and(|t| t.as_str().unwrap() == "two"));
    }

    #[test]
    fn test_store_insert_fluent() {
        assert!(Store::new()
            .with_must("three", "four")
            .get("three")
            .is_some_and(|t| t.as_str().unwrap() == "four"))
    }

    #[test]
    fn test_shadow_insert_and_get() {
        let mut store = Store::new();
        store.insert_must("one", "one");
        store.insert_must("two", "two");
        let mut shadow = Shadow::new(&store);
        // Push a frame here or the pop below will panic.
        shadow.push();
        shadow.insert_must("one", "shadowed one");

        assert_eq!(shadow.get("one"), Some(&json!("shadowed one")));
        assert_eq!(shadow.get("two"), Some(&json!("two")));
        shadow.pop();

        assert_eq!(shadow.get("one"), Some(&json!("one")));
        assert_eq!(shadow.get("two"), Some(&json!("two")));
    }

    #[test]
    #[should_panic(expected = "last scope must never be removed")]
    fn test_shadow_pop_empty() {
        let store = Store::new();
        let mut shadow = Shadow::new(&store);

        shadow.pop();
    }
}
