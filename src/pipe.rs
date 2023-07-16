use serde_json::{Map, Value};
use std::fmt::{Arguments, Display, Result, Write};

/// Wraps some underlying buffer by providing methods that write to it
/// in different formats.
pub struct Pipe<'buffer> {
    buffer: &'buffer mut (dyn Write + 'buffer),
}

impl<'buffer> Pipe<'buffer> {
    /// Create a new Pipe that writes to the given buffer.
    pub fn new(buffer: &'buffer mut String) -> Self {
        Self { buffer }
    }

    /// Write the given Value to the Pipe buffer.
    ///
    /// The Pipe will handle formatting the value.
    ///
    /// # Errors
    ///
    /// The Pipe supports all Value types, so the only error that will
    /// be returned is propogated from the [write!] macro itself.
    pub fn write_value(&mut self, value: &Value) -> Result {
        match value {
            Value::Null => self.write_null(),
            Value::String(string) => self.write_str(string),
            Value::Array(array) => self.write_array(array),
            Value::Object(object) => self.write_object(object),
            _ => self.write_display(value),
        }
    }

    /// Write the value to the buffer using the Display implementation.
    fn write_display(&mut self, value: impl Display) -> Result {
        write!(self.buffer, "{}", value)
    }

    /// Write the literal text "null" to the buffer.
    fn write_null(&mut self) -> Result {
        write!(self.buffer, "null")
    }

    /// Write the value to the buffer as a comma separated list and
    /// surrounded by braces.
    fn write_array(&mut self, value: &Vec<Value>) -> Result {
        write!(self.buffer, "[")?;
        let mut iter = value.iter();
        if let Some(item) = iter.next() {
            self.write_value(item)?;
            write!(self.buffer, ", ")?;
        }
        write!(self.buffer, "]")
    }

    /// Write the value to the buffer as key/value pairs and surrounded
    /// by curly braces.
    fn write_object(&mut self, value: &Map<String, Value>) -> Result {
        write!(self.buffer, "{{")?;
        let mut iter = value.iter();
        if let Some((key, value)) = iter.next() {
            write!(self.buffer, "{}: ", key)?;
            self.write_value(value)?;
            write!(self.buffer, ", ")?;
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
