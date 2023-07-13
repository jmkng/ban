use serde_json::Value;
use std::fmt::{Arguments, Write};

/// A wrapper around an underlying buffer which implements the Write trait
/// that provides methods to write various types in a desirable format.
pub struct Formatter<'a> {
    buffer: &'a mut (dyn Write + 'a),
}

impl<'a> Formatter<'a> {
    /// Create a new Formatter which writes to the given String.
    pub fn new(store: &'a mut String) -> Self {
        Self { buffer: store }
    }

    /// Write the given Value to the Formatter buffer.
    ///
    /// The Formatter will handle formatting the value.
    ///
    /// # Errors
    ///
    /// The Formatter supports all Value types, so the only error that will
    /// be returned is propogated from the [write!] macro itself.
    pub fn write_value(&mut self, value: &Value) -> std::fmt::Result {
        // TODO: Maybe this can be configurable somehow.
        match value {
            Value::Null => write!(self.buffer, "null"),
            Value::Bool(bool) => write!(self.buffer, "{}", bool),
            Value::Number(number) => write!(self.buffer, "{}", number),
            Value::String(string) => write!(self.buffer, "{}", string),
            Value::Array(array) => {
                write!(self.buffer, "[")?;
                let mut iter = array.iter();
                if let Some(item) = iter.next() {
                    self.write_value(item)?;
                    for item in iter {
                        write!(self.buffer, ", ")?;
                        self.write_value(item)?;
                    }
                }
                write!(self.buffer, "]")
            }
            Value::Object(object) => {
                write!(self.buffer, "{{")?;
                let mut iter = object.iter();
                if let Some((key, value)) = iter.next() {
                    write!(self.buffer, "{}: ", key)?;
                    self.write_value(value)?;
                    for (key, value) in iter {
                        write!(self.buffer, ", {}:", key)?;
                        self.write_value(value)?;
                    }
                }
                write!(self.buffer, "}}")
            }
        }
    }
}

impl Write for Formatter<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Write::write_str(self.buffer, s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        Write::write_char(self.buffer, c)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> std::fmt::Result {
        Write::write_fmt(self.buffer, args)
    }
}
