// # Sutra Atom System
//
// This module provides the atom system for the Sutra language engine.
// Atoms are the primitive operations that form the foundation of all computation.
//
// ## Module Structure
//
// - **`helpers`**: Shared infrastructure for all atoms
// - **`math`**: Mathematical operations (`+`, `-`, `*`, `/`, etc.)
// - **`logic`**: Logic and comparison operations (`eq?`, `not`, etc.)
// - **`world`**: World state management (`core/set!`, `core/get`, etc.)
// - **`collections`**: Collection operations (`list`, `len`, etc.)
// - **`execution`**: Execution control (`do`, `error`, `apply`)
// - **`external`**: External interface (`print`, `rand`)
//
// ## Design Principles
//
// - **Minimal Coupling**: Each domain module depends only on `helpers`
// - **Clear Responsibilities**: Each module has a single, well-defined purpose

// - **Consistent Interface**: All atoms use the same `AtomFn` signature

use crate::ast::value::Value;
use crate::ast::AstNode;
use crate::ast::Span;
use crate::runtime::eval::EvalContext;
use crate::runtime::context::ExecutionContext;
use crate::runtime::world::World;
use im::HashMap;
use crate::SutraError;
use crate::err_msg;

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

// ============================================================================
// NEW ATOM ARCHITECTURE TYPES
// ============================================================================

/// Pure atoms: operate only on values, no state access
pub type PureAtomFn = fn(args: &[Value]) -> Result<Value, SutraError>;

/// Stateful atoms: need limited state access via Context facade
pub type StatefulAtomFn =
    fn(args: &[Value], context: &mut ExecutionContext) -> Result<Value, SutraError>;

/// Special Form atoms: for atoms that need to control their own argument evaluation
pub type SpecialFormAtomFn = fn(
    args: &[AstNode],
    context: &mut EvalContext,
    parent_span: &Span,
) -> Result<(Value, World), SutraError>;

/// The unified atom representation supporting three calling conventions
#[derive(Clone)]
pub enum Atom {
    Pure(PureAtomFn),
    Stateful(StatefulAtomFn),
    SpecialForm(SpecialFormAtomFn),
}

/// Minimal state interface for stateful atoms
pub trait StateContext {
    fn get(&self, path: &crate::runtime::path::Path) -> Option<&Value>;
    fn set(&mut self, path: &crate::runtime::path::Path, value: Value);
    fn del(&mut self, path: &crate::runtime::path::Path);
    fn exists(&self, path: &crate::runtime::path::Path) -> bool;
}

/// Polymorphic invocation interface for all callable entities
/// This trait enables uniform invocation of atoms, macros, and future callable types
pub trait Callable {
    fn call(
        &self,
        args: &[Value],
        context: &mut ExecutionContext,
        current_world: &World,
    ) -> Result<(Value, World), SutraError>;
}

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
    pub atoms: HashMap<String, Atom>, // Changed from AtomFn to Atom
}

impl AtomRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&Atom> {
        // Changed return type
        self.atoms.get(name)
    }

    pub fn list(&self) -> Vec<String> {
        self.atoms.keys().cloned().collect()
    }

    // API for extensibility.
    pub fn register(&mut self, name: &str, func: Atom) {
        // Changed parameter type
        self.atoms.insert(name.to_string(), func);
    }

    pub fn clear(&mut self) {
        self.atoms.clear();
    }

    pub fn remove(&mut self, name: &str) -> Option<Atom> {
        // Changed return type
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
pub mod collections;
pub mod execution;
pub mod external;
pub mod logic;
pub mod math;
pub mod string;
pub mod world;

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
    string::register_string_atoms(registry);

    // Register test atoms only in debug or test builds
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    test::register_test_atoms(registry);
}

// ============================================================================
// CALLABLE TRAIT IMPLEMENTATIONS
// ============================================================================

impl Callable for Atom {
    fn call(
        &self,
        args: &[Value],
        context: &mut ExecutionContext,
        current_world: &World,
    ) -> Result<(Value, World), SutraError> {
        match self {
            Atom::Pure(pure_fn) => {
                let result = pure_fn(args)?;
                // Pure atoms don't modify world state, so return the current world unchanged
                Ok((result, current_world.clone()))
            }
            Atom::Stateful(stateful_fn) => {
                let result = stateful_fn(args, context)?;
                // The world is mutated in place via the context, so we just return it.
                Ok((result, current_world.clone()))
            }
            Atom::SpecialForm(_) => {
                // SpecialForm atoms require AstNode/EvalContext and cannot be called via Callable
                Err(err_msg!(Eval, "Special Form atoms cannot be called through Callable interface - use direct dispatch instead"))
            }
        }
    }
}
