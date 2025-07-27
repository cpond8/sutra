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

use std::{cell::RefCell, collections::HashMap, fmt, fs, rc::Rc, sync::Arc};

use miette::NamedSource;
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::macros::MacroEnvironment;

// Using a concrete, seedable PRNG for determinism.
type SmallRng = Xoshiro256StarStar;

// ============================================================================
// WORLD STATE - Simplified state management
// ============================================================================

/// A canonical, type-safe representation of a path into the world state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path(pub Vec<String>);

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

/// Simplified world state with root map structure
#[derive(Debug)]
pub struct WorldState {
    data: Value,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            data: Value::Map(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        let mut current = &self.data;
        for key in &path.0 {
            let Value::Map(map) = current else {
                return None;
            };
            let value = map.get(key.as_str())?;
            current = value;
        }
        Some(current)
    }

    pub fn set(&mut self, path: &Path, val: Value) {
        if path.0.is_empty() {
            return;
        }
        set_recursive_mut(&mut self.data, &path.0, val);
    }

    pub fn del(&mut self, path: &Path) {
        if path.0.is_empty() {
            return;
        }
        del_recursive_mut(&mut self.data, &path.0);
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// Top-level world container with state, PRNG, and macro environment
#[derive(Debug)]
pub struct World {
    pub state: WorldState,
    pub prng: SmallRng,
    pub macros: MacroEnvironment,
}

impl World {
    pub fn new() -> Self {
        let source = SourceContext::fallback("world state").to_named_source();
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_entropy(),
            macros: MacroEnvironment::new(source),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        let source = SourceContext::fallback("world state").to_named_source();
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_seed(seed),
            macros: MacroEnvironment::new(source),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        self.state.get(path)
    }

    pub fn set(&mut self, path: &Path, val: Value) {
        self.state.set(path, val);
    }

    pub fn del(&mut self, path: &Path) {
        self.state.del(path);
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
    }
}

/// Recursive helper for mutable set operations
fn set_recursive_mut(current: &mut Value, path_segments: &[String], val: Value) {
    let Some(key) = path_segments.first() else {
        return;
    };

    let remaining_segments = &path_segments[1..];

    // Ensure the current value is a map, upgrading if necessary.
    if !matches!(current, Value::Map(_)) {
        *current = Value::Map(HashMap::new());
    }

    let Value::Map(map) = current else {
        unreachable!(); // Should have been upgraded above
    };

    if remaining_segments.is_empty() {
        map.insert(key.clone(), val);
    } else {
        let child = map
            .entry(key.clone())
            .or_insert_with(|| Value::Map(HashMap::new()));
        set_recursive_mut(child, remaining_segments, val);
    }
}

/// Recursive helper for mutable delete operations
fn del_recursive_mut(current: &mut Value, path_segments: &[String]) {
    let Some(key) = path_segments.first() else {
        return;
    };

    let Value::Map(map) = current else {
        return; // Can't delete from a non-map value.
    };

    if path_segments.len() == 1 {
        map.remove(key);
    } else if let Some(child) = map.get_mut(key) {
        del_recursive_mut(child, &path_segments[1..]);
        // Clean up empty maps after deletion
        if let Value::Map(child_map) = child {
            if child_map.is_empty() {
                map.remove(key);
            }
        }
    }
}

/// Builds and returns a canonical, fully-populated world for evaluation.
pub fn build_canonical_world() -> CanonicalWorld {
    let mut world = World::new();
    register_all_atoms(&mut world);
    Rc::new(RefCell::new(world))
}

/// Builds and returns the canonical macro environment
pub fn build_canonical_macro_env() -> Result<MacroEnvironment, SutraError> {
    use crate::macros::{load_macros_from_source, std_macros::register_std_macros};

    // Step 1: Create environment and register standard macros
    let source = Arc::new(NamedSource::new(
        "std_macros.sutra",
        fs::read_to_string("src/macros/std_macros.sutra").unwrap_or_default(),
    ));
    let mut env = MacroEnvironment::new(source);
    register_std_macros(&mut env);

    // Step 2: Load user macros from file
    let file_content = fs::read_to_string("src/macros/std_macros.sutra").unwrap_or_default();
    load_macros_from_source(&file_content, &mut env)?;

    Ok(env)
}

// ============================================================================
// STATE CONTEXT IMPLEMENTATION
// ============================================================================

impl StateContext for WorldState {
    fn get(&self, path: &Path) -> Option<&Value> {
        self.get(path)
    }

    fn set(&mut self, path: &Path, value: Value) {
        set_recursive_mut(&mut self.data, &path.0, value);
    }

    fn del(&mut self, path: &Path) {
        del_recursive_mut(&mut self.data, &path.0);
    }

    fn exists(&self, path: &Path) -> bool {
        self.get(path).is_some()
    }
}

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

// Domain-specific atom modules
pub mod collections;
pub mod execution;
pub mod external;
pub mod logic;
pub mod math;
pub mod special_forms;
pub mod string;
pub mod world;

// Re-export output types for external use
pub use external::{EngineOutputBuffer, EngineStdoutSink};

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
        $world.set(&Path(vec![$name.to_string()]), Value::NativeFn($func));
    };
}

#[macro_export]
macro_rules! register_lazy {
    ($world:expr, $name:expr, $func:expr) => {
        $world.set(&Path(vec![$name.to_string()]), Value::NativeFn($func));
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
    register_eager!(world, "append", collections::ATOM_APPEND);
    register_eager!(world, "map", collections::ATOM_MAP);
    register_eager!(world, "core/str+", collections::ATOM_CORE_STR_PLUS);
    register_eager!(world, "core/map", collections::ATOM_CORE_MAP);
}

fn register_execution_atoms(world: &mut World) {
    register_lazy!(world, "do", execution::ATOM_DO);
    register_lazy!(world, "error", execution::ATOM_ERROR);
    register_lazy!(world, "apply", execution::ATOM_APPLY);
    register_lazy!(world, "for-each", execution::ATOM_FOR_EACH);
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
    register_lazy!(world, "cond", special_forms::ATOM_COND);
    register_lazy!(world, "and", special_forms::ATOM_AND);
    register_lazy!(world, "or", special_forms::ATOM_OR);
    register_lazy!(world, "define", special_forms::ATOM_DEFINE);
}
