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
//
// ## CRITICAL: Atom Classification and Calling Conventions
//
// The Sutra engine supports two incompatible calling conventions for atoms,
// determined by the `Atom` enum variant used at registration:
//
// 1.  **Eager Evaluation (`Pure`, `Stateful`)**: Arguments are evaluated *before*
//     the atom is called. The atom receives a `&[Value]`. This is the standard
//     convention for most atoms.
//
// 2.  **Lazy Evaluation (`SpecialForm`)**: Arguments are passed *unevaluated* as
//     `&[AstNode]`. The atom is responsible for evaluating them as needed. This
//     is used for control flow operators like `if`, `do`, and `define`.
//
// **Misclassifying an atom will cause immediate runtime failures.** For example,
// registering a `SpecialForm` atom as `Pure` will lead to incorrect evaluation
// and likely panic. Debug assertions have been added to `eval::call_atom` to
// catch such misclassifications for known special forms.

use crate::ast::value::Value;
use crate::ast::AstNode;
use crate::ast::Span;
use crate::runtime::eval::EvaluationContext;
use crate::runtime::world::AtomExecutionContext;
use crate::runtime::world::World;
use im::HashMap;
use crate::SutraError;
use crate::err_msg;
use std::rc::Rc;
use std::cell::RefCell;

// ============================================================================
// NEW ATOM ARCHITECTURE TYPES
// ============================================================================
//
// The `Atom` enum and its associated function types are the foundation of the
// dual-convention architecture. Correct classification is critical for stability.

/// Pure atoms: operate only on values, no state access
pub type PureAtomFn = fn(args: &[Value]) -> Result<Value, SutraError>;

/// Stateful atoms: need limited state access via Context facade
pub type StatefulAtomFn =
    fn(args: &[Value], context: &mut AtomExecutionContext) -> Result<Value, SutraError>;

/// Special Form atoms: for atoms that need to control their own argument evaluation
pub type SpecialFormAtomFn = fn(
    args: &[AstNode],
    context: &mut EvaluationContext,
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
    fn get(&self, path: &crate::runtime::world::Path) -> Option<&Value>;
    fn set(&mut self, path: &crate::runtime::world::Path, value: Value);
    fn del(&mut self, path: &crate::runtime::world::Path);
    fn exists(&self, path: &crate::runtime::world::Path) -> bool;
}

/// Polymorphic invocation interface for all callable entities
/// This trait enables uniform invocation of atoms, macros, and future callable types
pub trait Callable {
    fn call(
        &self,
        args: &[Value],
        context: &mut AtomExecutionContext,
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

/// Ergonomic, extensible wrapper for shared, mutable output sinks.
#[derive(Clone)]
pub struct SharedOutput(pub Rc<RefCell<dyn OutputSink>>);

impl SharedOutput {
    /// Create a new SharedOutput from any OutputSink.
    pub fn new<T: OutputSink + 'static>(sink: T) -> Self {
        SharedOutput(Rc::new(RefCell::new(sink)))
    }
    /// Emit output via the sink.
    pub fn emit(&self, text: &str, span: Option<&crate::ast::Span>) {
        self.0.borrow_mut().emit(text, span);
    }
    /// Borrow the sink mutably (for advanced use).
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, dyn OutputSink> {
        self.0.borrow_mut()
    }
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
pub mod special_forms;

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
    register_special_forms(registry);

    // Register test atoms only in debug or test builds
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    test::register_test_atoms(registry);
}

/// Registers all collection atoms with the given registry.
///
/// List operations: list, len, has?, car, cdr, cons, core/push!, core/pull!
/// String operations: core/str+
pub fn register_collection_atoms(registry: &mut AtomRegistry) {
    // List operations
    registry.register("list", Atom::Pure(collections::ATOM_LIST));
    registry.register("len", Atom::Pure(collections::ATOM_LEN));
    registry.register("has?", Atom::Pure(collections::ATOM_HAS));
    registry.register("car", Atom::Pure(collections::ATOM_CAR));
    registry.register("cdr", Atom::Pure(collections::ATOM_CDR));
    registry.register("cons", Atom::Pure(collections::ATOM_CONS));

    // String operations
    registry.register("core/str+", Atom::Pure(collections::ATOM_CORE_STR_PLUS));
}

/// Registers all special form atoms (lambda, let, if, etc.) with the given registry.
pub fn register_special_forms(registry: &mut AtomRegistry) {
    registry.register("lambda", Atom::SpecialForm(special_forms::ATOM_LAMBDA));
    registry.register("let", Atom::SpecialForm(special_forms::ATOM_LET));
    registry.register("if", Atom::SpecialForm(special_forms::ATOM_IF));
}

// ============================================================================
// CALLABLE TRAIT IMPLEMENTATIONS
// ============================================================================

impl Callable for Atom {
    fn call(
        &self,
        args: &[Value],
        context: &mut AtomExecutionContext,
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
                // SpecialForm atoms require AstNode/EvaluationContext and cannot be called via Callable
                Err(err_msg!(Eval, "Special Form atoms cannot be called through Callable interface - use direct dispatch instead"))
            }
        }
    }
}
