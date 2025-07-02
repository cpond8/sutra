use crate::path::Path;
use im::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Number(f64),
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Path(Path),
}

impl Value {
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

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}
