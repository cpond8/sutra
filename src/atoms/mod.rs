//! # Sutra Atom System
//!
//! This module provides the atom system for the Sutra language engine.
//! Atoms are the primitive operations that form the foundation of all computation.
//!
//! ## Module Structure
//!
//! - **`helpers`**: Shared infrastructure for all atoms
//! - **`math`**: Mathematical operations (`+`, `-`, `*`, `/`, etc.)
//! - **`logic`**: Logic and comparison operations (`eq?`, `not`, etc.)
//! - **`world`**: World state management (`core/set!`, `core/get`, etc.)
//! - **`collections`**: Collection operations (`list`, `len`, etc.)
//! - **`execution`**: Execution control (`do`, `error`, `apply`)
//! - **`external`**: External interface (`print`, `rand`)
//!
//! ## Design Principles
//!
//! - **Minimal Coupling**: Each domain module depends only on `helpers`
//! - **Clear Responsibilities**: Each module has a single, well-defined purpose
//! - **Consistent Interface**: All atoms use the same `AtomFn` signature

use crate::ast::value::Value;
use crate::ast::{Span};
use crate::runtime::eval::EvalContext;
use crate::runtime::world::World;
use crate::syntax::error::SutraError;
use im::HashMap;
use crate::ast::AstNode;

// ============================================================================
// CORE TYPES AND TRAITS
// ============================================================================

// Atom function type: takes AST arguments, the current evaluation context,
// and the span of the parent expression for better error reporting.
// It returns a tuple containing the resulting Value and the new World state,
// ensuring that all state changes are explicit and pure.
pub type AtomFn = fn(
    args: &[AstNode],
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

    pub fn clear(&mut self) {
        self.atoms.clear();
    }

    pub fn remove(&mut self, name: &str) -> Option<AtomFn> {
        self.atoms.remove(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.atoms.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.atoms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }
}

// ============================================================================
// MODULAR ATOM IMPLEMENTATIONS
// ============================================================================

// Core infrastructure shared by all atoms
pub mod helpers;

// Domain-specific atom modules
pub mod math;
pub mod logic;
pub mod world;
pub mod collections;
pub mod execution;
pub mod external;

// Test atoms module - only available in debug/test builds
#[cfg(any(test, feature = "test-atom", debug_assertions))]
pub mod test;

// ============================================================================
// UNIFIED REGISTRATION FUNCTION
// ============================================================================

/// Registers all standard atoms from all modules with the given registry.
/// This is the main entry point for setting up the complete atom system.
pub fn register_all_atoms(registry: &mut AtomRegistry) {
    // Register atoms from each domain module
    math::register_math_atoms(registry);
    logic::register_logic_atoms(registry);
    world::register_world_atoms(registry);
    collections::register_collection_atoms(registry);
    execution::register_execution_atoms(registry);
    external::register_external_atoms(registry);
}
