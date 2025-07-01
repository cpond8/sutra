use crate::ast::{Expr, Span};
use crate::error::SutraError;
use crate::eval::EvalContext;
use crate::value::Value;
use crate::world::World;
use std::collections::HashMap;

// Atom function type: takes AST arguments and the current evaluation context.
// It returns a tuple containing the resulting Value and the new World state,
// ensuring that all state changes are explicit and pure.
pub type AtomFn =
    fn(args: &[Expr], context: &mut EvalContext) -> Result<(Value, World), SutraError>;

// Output sink for `print`, etc., to make I/O testable and injectable.
pub trait OutputSink {
    fn emit(&mut self, text: &str, span: Option<&Span>);
}

// A null output sink for testing or running without output.
pub struct NullSink;
impl OutputSink for NullSink {
    fn emit(&mut self, _text: &str, _span: Option<&Span>) {}
}

// Registry for all atoms, inspectable at runtime.
#[derive(Default)]
pub struct AtomRegistry {
    pub atoms: HashMap<String, AtomFn>,
}

impl AtomRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&AtomFn> {
        self.atoms.get(name)
    }

    pub fn list(&self) -> Vec<String> {
        self.atoms.keys().cloned().collect()
    }

    // API for extensibility.
    pub fn register(&mut self, name: &str, func: AtomFn) {
        self.atoms.insert(name.to_string(), func);
    }
}
