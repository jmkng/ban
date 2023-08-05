use serde_json::{Map, Value};
use std::fmt::{Arguments, Display, Result, Write};

/// Wraps some underlying buffer by providing methods that write to it
/// in different formats.
pub struct Pipe<'buffer> {
    buffer: &'buffer mut (dyn Write + 'buffer),
}

impl<'buffer> Pipe<'buffer> {
    /// Create a new `Pipe` that writes to the given buffer.
    pub fn new(buffer: &'buffer mut String) -> Self {
        Self { buffer }
    }

    /// Write the given [`Value`] to the [`Pipe`] buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`][`std::fmt::Error`] if the `write!` macro
    /// is unable to write the `Value`.
    pub fn write_value(&mut self, value: &Value) -> Result {
        match value {
            Value::Null => Ok(()),
            Value::String(string) => self.write_str(string),
            Value::Array(array) => self.write_array(array),
            Value::Object(object) => self.write_object(object),
            _ => self.write_display(value),
        }
    }

    /// Write the [`Value`] to the buffer using the Display implementation.
    fn write_display(&mut self, value: impl Display) -> Result {
        write!(self.buffer, "{}", value)
    }

    /// Write the [`Value`] to the buffer as a comma separated list and
    /// surrounded by square brackets.
    fn write_array(&mut self, value: &Vec<Value>) -> Result {
        write!(self.buffer, "[")?;
        let mut iterator = value.iter().peekable();
        while let Some(item) = iterator.next() {
            self.write_value(item)?;
            if iterator.peek().is_some() {
                write!(self.buffer, ", ")?;
            }
        }

        write!(self.buffer, "]")
    }

    /// Write the [`Value`] to the buffer as a key-value pair and surrounded
    /// by curly braces.
    fn write_object(&mut self, value: &Map<String, Value>) -> Result {
        write!(self.buffer, "{{")?;
        let mut iterator = value.iter().peekable();
        while let Some((key, value)) = iterator.next() {
            write!(self.buffer, "{key}: ")?;
            self.write_value(value)?;
            if iterator.peek().is_some() {
                write!(self.buffer, ", ")?;
            }
        }

        write!(self.buffer, "}}")
    }
}

impl Write for Pipe<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result {
        Write::write_str(self.buffer, s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result {
        Write::write_char(self.buffer, c)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        Write::write_fmt(self.buffer, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_write_value_string() {
        let mut buffer = String::new();
        let mut pipe = Pipe::new(&mut buffer);
        pipe.write_value(&json!("Hello, World!")).unwrap();

        assert_eq!(buffer, "Hello, World!");
    }

    #[test]
    fn test_write_value_array() {
        let mut buffer = String::new();
        let mut pipe = Pipe::new(&mut buffer);
        pipe.write_value(&json!([1, 2, 3])).unwrap();

        assert_eq!(buffer, "[1, 2, 3]");
    }

    #[test]
    fn test_write_value_object() {
        let mut buffer = String::new();
        let mut pipe = Pipe::new(&mut buffer);
        pipe.write_value(&json!({"one": "two", "three": "four"}))
            .unwrap();

        assert_eq!(buffer, "{one: two, three: four}");
    }
}
