// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::value::Value;
use crate::ast::{Expr, Span, WithSpan};
use crate::runtime::eval::EvalContext;
use crate::runtime::world::World;
use crate::syntax::error::SutraError;
use im::HashMap;

// Atom function type: takes AST arguments, the current evaluation context,
// and the span of the parent expression for better error reporting.
// It returns a tuple containing the resulting Value and the new World state,
// ensuring that all state changes are explicit and pure.
pub type AtomFn = fn(
    args: &[WithSpan<Expr>],
    context: &mut EvalContext,
    parent_span: &Span,
) -> Result<(Value, World), SutraError>;

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

pub mod std;

// Test atoms module - only available in debug/test builds
#[cfg(any(test, feature = "test-atom", debug_assertions))]
pub mod test;
