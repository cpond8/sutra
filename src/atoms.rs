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
// ## Native Function Calling Conventions
//
// All built-in functions ("atoms") are first-class `Value` types, specifically
// `Value::NativeEagerFn` and `Value::NativeLazyFn`. This design retains the dual
// calling conventions critical for the language's semantics:
//
// 1.  **Eager Functions (`NativeEagerFn`)**: Arguments are evaluated *before* the
//     function is called. The native Rust function receives a `&[Value]`. This is
//     the standard convention for most operations (e.g., `+`, `eq?`).
//
// 2.  **Lazy Functions (`NativeLazyFn`)**: Arguments are passed *unevaluated* as
//     `&[AstNode]`. The function itself controls if and when to evaluate its
//     arguments. This is essential for special forms that manage control flow
//     (e.g., `if`, `lambda`, `let`).

use std::{cell::RefCell, rc::Rc};


// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{
    atoms::helpers::AtomResult,
};

// ============================================================================
// NEW ATOM ARCHITECTURE TYPES
// ============================================================================
//
// All callable entities are now dispatched through `evaluate_list` in `eval.rs`,
// which inspects the `Value` type to determine the correct calling convention.


/// Minimal state interface for stateful atoms
pub trait StateContext {
    fn get(&self, path: &Path) -> Option<&Value>;
    fn set(&mut self, path: &Path, value: Value);
    fn del(&mut self, path: &Path);
    fn exists(&self, path: &Path) -> bool;
}

// The `Callable` trait has been removed. All callable entities (native functions,
// lambdas) are resolved to a `Value` and then handled by `evaluate_list`, which
// acts as the single dispatch point. This ensures each callable type receives
// the correct arguments and evaluation semantics.

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
    pub fn emit(&self, text: &str, span: Option<&Span>) {
        self.0.borrow_mut().emit(text, span);
    }
    /// Borrow the sink mutably (for advanced use).
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, dyn OutputSink> {
        self.0.borrow_mut()
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
pub mod special_forms;
pub mod string;
pub mod world;

// Test atoms module - only available in debug/test builds
#[cfg(any(test, feature = "test-atom", debug_assertions))]
pub mod test;

// Re-exports for concise imports
#[cfg(any(test, feature = "test-atom", debug_assertions))]
pub use test::{TestDefinition, TEST_REGISTRY};

// ============================================================================
// UNIFIED REGISTRATION FUNCTION
// ============================================================================

/// Registers all standard native functions from all modules into the given `World`.
/// This is the main entry point for populating the global environment.
// Helper macro to reduce boilerplate in registration
#[macro_export]
macro_rules! register_eager {
    ($world:expr, $name:expr, $func:expr) => {
        $world.set(
            &Path(vec![$name.to_string()]),
            Value::NativeEagerFn($func),
        );
    };
}

#[macro_export]
macro_rules! register_lazy {
    ($world:expr, $name:expr, $func:expr) => {
        $world.set(
            &Path(vec![$name.to_string()]),
            Value::NativeLazyFn($func),
        );
    };
}

/// Registers all standard atoms from all modules with the given world.
pub fn register_all_atoms(world: &mut World) {
    // Register atoms from each domain module
    register_math_atoms(world);
    register_logic_atoms(world);
    register_world_atoms(world);
    register_collection_atoms(world);
    register_execution_atoms(world);
    register_external_atoms(world);
    register_string_atoms(world);
    register_special_forms(world);

    // Register test atoms only in debug or test builds
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    test::register_test_atoms(world);
}

// Private registration functions for each module
fn register_math_atoms(world: &mut World) {
    register_eager!(world, "+", math::ATOM_ADD);
    register_eager!(world, "-", math::ATOM_SUB);
    register_eager!(world, "*", math::ATOM_MUL);
    register_eager!(world, "/", math::ATOM_DIV);
    register_eager!(world, "mod", math::ATOM_MOD);
    register_eager!(world, "abs", math::ATOM_ABS);
    register_eager!(world, "min", math::ATOM_MIN);
    register_eager!(world, "max", math::ATOM_MAX);
}

fn register_logic_atoms(world: &mut World) {
    register_eager!(world, "eq?", logic::ATOM_EQ);
    register_eager!(world, "gt?", logic::ATOM_GT);
    register_eager!(world, "lt?", logic::ATOM_LT);
    register_eager!(world, "gte?", logic::ATOM_GTE);
    register_eager!(world, "lte?", logic::ATOM_LTE);
    register_eager!(world, "is?", logic::ATOM_EQ);
    register_eager!(world, "over?", logic::ATOM_GT);
    register_eager!(world, "under?", logic::ATOM_LT);
    register_eager!(world, "at-least?", logic::ATOM_GTE);
    register_eager!(world, "at-most?", logic::ATOM_LTE);
    register_eager!(world, "not", logic::ATOM_NOT);
}

fn register_world_atoms(world: &mut World) {
    register_eager!(world, "core/set!", world::ATOM_CORE_SET);
    register_eager!(world, "core/get", world::ATOM_CORE_GET);
    register_eager!(world, "core/del!", world::ATOM_CORE_DEL);
    register_eager!(world, "core/exists?", world::ATOM_EXISTS);
    register_eager!(world, "path", world::ATOM_PATH);
}

fn register_collection_atoms(world: &mut World) {
    register_eager!(world, "list", collections::ATOM_LIST);
    register_eager!(world, "len", collections::ATOM_LEN);
    register_eager!(world, "has?", collections::ATOM_HAS);
    register_eager!(world, "car", collections::ATOM_CAR);
    register_eager!(world, "cdr", collections::ATOM_CDR);
    register_eager!(world, "cons", collections::ATOM_CONS);
    register_eager!(world, "core/push!", collections::ATOM_CORE_PUSH);
    register_eager!(world, "core/pull!", collections::ATOM_CORE_PULL);
    register_eager!(world, "core/str+", collections::ATOM_CORE_STR_PLUS);
    register_eager!(world, "core/map", collections::ATOM_CORE_MAP);
}

fn register_execution_atoms(world: &mut World) {
    register_lazy!(world, "do", execution::ATOM_DO);
    register_lazy!(world, "error", execution::ATOM_ERROR);
    register_lazy!(world, "apply", execution::ATOM_APPLY);
}

fn register_external_atoms(world: &mut World) {
    register_eager!(world, "print", external::ATOM_PRINT);
    register_eager!(world, "core/print", external::ATOM_PRINT);
    register_eager!(world, "output", external::ATOM_OUTPUT);
    register_eager!(world, "rand", external::ATOM_RAND);
}

fn register_string_atoms(world: &mut World) {
    register_eager!(world, "str", string::ATOM_STR);
    register_eager!(world, "str+", string::ATOM_STR_PLUS);
}

fn register_special_forms(world: &mut World) {
    register_lazy!(world, "lambda", special_forms::ATOM_LAMBDA);
    register_lazy!(world, "let", special_forms::ATOM_LET);
    register_lazy!(world, "if", special_forms::ATOM_IF);
    register_lazy!(world, "define", special_forms::ATOM_DEFINE);
}
