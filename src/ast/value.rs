use crate::runtime::path::Path;
use im::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a value in the Sutra engine.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::value::Value;
/// let n = Value::Number(3.14);
/// assert_eq!(n.type_name(), "Number");
/// let s = Value::String("hello".to_string());
/// assert_eq!(s.type_name(), "String");
/// let nil = Value::default();
/// assert!(nil.is_nil());
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum Value {
    #[default]
    Nil,
    Number(f64),
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Path(Path),
}

impl Value {
    /// Returns the type name of the value as a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::value::Value;
    /// let v = Value::Bool(true);
    /// assert_eq!(v.type_name(), "Bool");
    /// ```
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Path(_) => "Path",
        }
    }

    /// Returns true if the value is Nil.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::value::Value;
    /// assert!(Value::Nil.is_nil());
    /// assert!(!Value::Number(1.0).is_nil());
    /// ```
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Returns the contained number if this is a Number value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::value::Value;
    /// let v = Value::Number(2.0);
    /// assert_eq!(v.as_number(), Some(2.0));
    /// let v2 = Value::String("nope".to_string());
    /// assert_eq!(v2.as_number(), None);
    /// ```
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the contained bool if this is a Bool value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::value::Value;
    /// let v = Value::Bool(false);
    /// assert_eq!(v.as_bool(), Some(false));
    /// let v2 = Value::Nil;
    /// assert_eq!(v2.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    // ------------------------------------------------------------------------
    // Display formatting helpers
    // ------------------------------------------------------------------------

    /// Helper for formatting list values
    fn fmt_list(f: &mut fmt::Formatter<'_>, items: &[Value]) -> fmt::Result {
        write!(f, "(")?;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, ")")
    }

    /// Helper for formatting map values
    fn fmt_map(f: &mut fmt::Formatter<'_>, map: &HashMap<String, Value>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in map.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", k, v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => Value::fmt_list(f, items),
            Value::Map(map) => Value::fmt_map(f, map),
            Value::Path(p) => write!(f, "{}", p),
        }
    }
}
