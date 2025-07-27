//! Canonical runtime value type for the Sutra engine.
//!
//! All computation, macro expansion, and evaluation in Sutra produces or manipulates a `Value`.
//! This type is deeply compositional: lists and maps can contain any other value, including nested lists and maps.
//! `Nil` represents the absence of a value and is the default for all uninitialized slots.
//! `Path` is a first-class value, enabling explicit reference to locations in the world state.

use std::{collections::HashMap, fmt, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::{
    atoms::helpers::AtomResult, engine::EvaluationContext, AstNode, ParamList, Path, Span,
};

/// Eagerly evaluated native function: receives evaluated arguments.
pub type NativeEagerFn = fn(args: &[Value], context: &mut EvaluationContext) -> AtomResult;

/// Lazily evaluated native function: receives unevaluated AST nodes.
pub type NativeLazyFn =
    fn(args: &[AstNode], context: &mut EvaluationContext, parent_span: &Span) -> AtomResult;

/// Canonical runtime value for Sutra evaluation and macro expansion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Value {
    /// Absence of a value; default for uninitialized slots.
    #[default]
    Nil,
    /// Numeric value (floating point).
    Number(f64),
    /// String value.
    String(String),
    /// Boolean value.
    Bool(bool),
    /// A Lisp-style cons cell, forming the head of a list.
    Cons(Rc<ConsCell>),
    /// Map from string keys to values (deeply compositional).
    Map(HashMap<String, Value>),
    /// Reference to a path in the world state (not auto-resolved).
    Path(Path),
    /// User-defined lambda function (captures parameter list and body).
    Lambda(Rc<Lambda>),
    /// Native (Rust) function that evaluates its arguments eagerly.
    #[serde(skip)]
    NativeEagerFn(NativeEagerFn),
    /// Native (Rust) function that evaluates its arguments lazily.
    #[serde(skip)]
    NativeLazyFn(NativeLazyFn),
    Symbol(String),
    Quote(Box<Value>),
}

/// Represents a user-defined lambda function (for closures and function values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lambda {
    pub params: ParamList,  // Parameter names, variadic info
    pub body: Box<AstNode>, // The function body (AST)
    pub captured_env: HashMap<String, Value>,
}

/// Represents a Lisp-style cons cell (a pair of values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsCell {
    pub car: Value,
    pub cdr: Value,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Cons(a), Value::Cons(b)) => Rc::ptr_eq(a, b) || a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Path(a), Value::Path(b)) => a == b,
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (Value::NativeEagerFn(a), Value::NativeEagerFn(b)) => *a as usize == *b as usize,
            (Value::NativeLazyFn(a), Value::NativeLazyFn(b)) => *a as usize == *b as usize,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Quote(a), Value::Quote(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    /// Returns the type name of the value as a string (for diagnostics, debugging, and macro logic).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Cons(_) => "List", // User-facing type name remains "List" for consistency.
            Value::Map(_) => "Map",
            Value::Path(_) => "Path",
            Value::Lambda(_) => "Lambda",
            Value::NativeEagerFn(_) => "NativeEagerFn",
            Value::NativeLazyFn(_) => "NativeLazyFn",
            Value::Symbol(_) => "Symbol",
            Value::Quote(_) => "Quote",
        }
    }

    /// Returns true if the value is Nil (used for default checks and absence semantics).
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Returns true if the value is considered "truthy" in a boolean context.
    /// In Sutra, `nil` and `false` are falsey. Everything else is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    /// Returns the contained number if this is a Number value, else None.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the contained bool if this is a Bool value, else None.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the contained string if this is a String value, else None.
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns a reference to the contained string if this is a String value, else None.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns a reference to the contained map if this is a Map value, else None.
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    // ------------------------------------------------------------------------
    // Display formatting helpers (internal)
    // ------------------------------------------------------------------------

    fn fmt_cons_chain(f: &mut fmt::Formatter<'_>, start_cell: &ConsCell) -> fmt::Result {
        write!(f, "(")?;
        write!(f, "{}", start_cell.car)?;

        let mut current_cdr = &start_cell.cdr;
        loop {
            match current_cdr {
                // Proper list case: the cdr is another cons cell
                Value::Cons(next_cell) => {
                    write!(f, " {}", next_cell.car)?;
                    current_cdr = &next_cell.cdr;
                }
                // Proper list termination
                Value::Nil => {
                    break;
                }
                // Improper list case (dotted pair)
                other => {
                    write!(f, " . {}", other)?;
                    break;
                }
            }
        }
        write!(f, ")")
    }

    fn fmt_map(f: &mut fmt::Formatter<'_>, map: &HashMap<String, Value>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in map.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
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
                    write!(f, "{n}")
                }
            }
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Cons(cell) => Value::fmt_cons_chain(f, cell),
            Value::Map(map) => Value::fmt_map(f, map),
            Value::Path(p) => write!(f, "{p}"),
            Value::Lambda(_) => write!(f, "<lambda>"),
            Value::NativeEagerFn(_) => write!(f, "<native_eager_fn>"),
            Value::NativeLazyFn(_) => write!(f, "<native_lazy_fn>"),
            Value::Symbol(s) => write!(f, "{s}"),
            Value::Quote(v) => write!(f, "'{v}"),
        }
    }
}
