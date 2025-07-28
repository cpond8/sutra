// # Sutra Atom System
//
// This module provides the core atom system for the Sutra language engine.
// Atoms are built-in primitive functions that form the foundation of all computation
// in the Sutra language.
//
// ## Module Structure
//
// The atom system is organized into domain-specific modules:
//
// - **`math`**: Arithmetic operations (`+`, `-`, `*`, `/`, `mod`, `abs`, `min`, `max`)
// - **`logic`**: Logic and comparison operations (`eq?`, `gt?`, `lt?`, `not`, etc.)
// - **`collections`**: List and collection operations (`list`, `len`, `null?`, `car`, `cdr`, etc.)
// - **`world`**: World state management (`set!`, `get`, `del!`, `exists?`, etc.)
// - **`execution`**: Control flow and execution (`do`, `error`, `apply`, `for-each`)
// - **`external`**: External I/O operations (`print`, `println`, `output`, `rand`)
// - **`string`**: String manipulation (`str`, `str+`)
// - **`special_forms`**: Language special forms (`lambda`, `let`, `if`, `cond`, `and`, `or`, `define`)
//
// ## Architecture
//
// ### Native Functions
// All atoms are implemented as native Rust functions with the signature `NativeFn`:
// ```rust
// type NativeFn = fn(&[AstNode], &mut EvaluationContext, &Span) -> SpannedResult;
// ```
//
// ### Registration System
// Atoms are registered into the global environment using the `register_atom!` macro.
// All atoms use the same unified `NativeFn` signature regardless of whether they
// control their own argument evaluation (like special forms) or expect pre-evaluated
// arguments (like arithmetic operations).
//
// ### Aliases
// Many comparison and mathematical operations support multiple aliases for convenience:
// - `eq?`, `=`, `is?` all refer to equality comparison
// - `gt?`, `>`, `over?` all refer to greater-than comparison
// - `lt?`, `<`, `under?` all refer to less-than comparison
// - And so on for `>=`, `<=`, etc.
//
// ## World State Management
//
// The atom system includes a sophisticated world state management system that allows
// atoms to read from and modify persistent state. The `World` struct contains:
//
// - **State**: A hierarchical key-value store accessible via dot-notation paths
// - **PRNG**: A seedable random number generator for deterministic randomness
// - **Macros**: A macro expansion system for code transformation

use std::{cell::RefCell, collections::HashMap, fmt, fs, rc::Rc};

use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::macros::MacroSystem;

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
    pub macros: MacroSystem,
}

impl World {
    pub fn new() -> Self {
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_entropy(),
            macros: MacroSystem::new(),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_seed(seed),
            macros: MacroSystem::new(),
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
pub fn build_canonical_macro_env() -> Result<MacroSystem, SutraError> {
    // Step 1: Create environment (standard macros are auto-registered)
    let mut env = MacroSystem::new();

    // Step 2: Load user macros from file
    let file_content = fs::read_to_string("src/macros/std_macros.sutra").unwrap_or_default();
    env.load_from_source(&file_content)?;

    Ok(env)
}

// ============================================================================
// STATE AND OUTPUT INFRASTRUCTURE
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

/// Minimal state interface for stateful atoms
pub trait StateContext {
    fn get(&self, path: &Path) -> Option<&Value>;
    fn set(&mut self, path: &Path, value: Value);
    fn del(&mut self, path: &Path);
    fn exists(&self, path: &Path) -> bool;
}

/// Output sink for `print` and similar I/O operations, enabling testable and injectable output.
pub trait OutputSink {
    fn emit(&mut self, text: &str, span: Option<&Span>);
}

/// A null output sink for testing or running without output.
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
pub mod world;

// Re-export output types for external use
pub use external::{EngineOutputBuffer, EngineStdoutSink};

// Test atoms module - only available in debug/test builds
#[cfg(any(test, feature = "test-atom", debug_assertions))]
pub mod test;

// ============================================================================
// ATOM REGISTRATION SYSTEM
// ============================================================================

/// Helper macro to register atoms in the global environment.
/// All atoms use the same `NativeFn` signature regardless of evaluation strategy.
#[macro_export]
macro_rules! register_atom {
    ($world:expr, $name:expr, $func:expr) => {
        $world.set(&Path(vec![$name.to_string()]), Value::NativeFn($func));
    };
}

/// Registers all standard atoms from all modules with the given world.
/// This is the main entry point for populating the global environment.
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
    register_atom!(world, "+", math::ATOM_ADD);
    register_atom!(world, "-", math::ATOM_SUB);
    register_atom!(world, "*", math::ATOM_MUL);
    register_atom!(world, "/", math::ATOM_DIV);
    register_atom!(world, "mod", math::ATOM_MOD);
    register_atom!(world, "abs", math::ATOM_ABS);
    register_atom!(world, "min", math::ATOM_MIN);
    register_atom!(world, "max", math::ATOM_MAX);
}

fn register_logic_atoms(world: &mut World) {
    register_atom!(world, "eq?", logic::ATOM_EQ);
    register_atom!(world, "=", logic::ATOM_EQ); // alias for eq?
    register_atom!(world, "is?", logic::ATOM_EQ); // alias for eq?
    register_atom!(world, "gt?", logic::ATOM_GT);
    register_atom!(world, ">", logic::ATOM_GT); // alias for gt?
    register_atom!(world, "over?", logic::ATOM_GT); // alias for gt?
    register_atom!(world, "lt?", logic::ATOM_LT);
    register_atom!(world, "<", logic::ATOM_LT); // alias for lt?
    register_atom!(world, "under?", logic::ATOM_LT); // alias for lt?
    register_atom!(world, "gte?", logic::ATOM_GTE);
    register_atom!(world, ">=", logic::ATOM_GTE); // alias for gte?
    register_atom!(world, "at-least?", logic::ATOM_GTE); // alias for gte?
    register_atom!(world, "lte?", logic::ATOM_LTE);
    register_atom!(world, "<=", logic::ATOM_LTE); // alias for lte?
    register_atom!(world, "at-most?", logic::ATOM_LTE); // alias for lte?
    register_atom!(world, "not", logic::ATOM_NOT);
}

fn register_world_atoms(world: &mut World) {
    register_atom!(world, "set!", world::ATOM_SET);
    register_atom!(world, "get", world::ATOM_GET);
    register_atom!(world, "del!", world::ATOM_DEL);
    register_atom!(world, "exists?", world::ATOM_EXISTS);
    register_atom!(world, "inc!", world::ATOM_INC);
    register_atom!(world, "dec!", world::ATOM_DEC);
    register_atom!(world, "add!", world::ATOM_ADD);
    register_atom!(world, "sub!", world::ATOM_SUB);
    register_atom!(world, "path", world::ATOM_PATH);
}

fn register_collection_atoms(world: &mut World) {
    register_atom!(world, "list", collections::ATOM_LIST);
    register_atom!(world, "len", collections::ATOM_LEN);
    register_atom!(world, "null?", collections::ATOM_NULL);
    register_atom!(world, "has?", collections::ATOM_HAS);
    register_atom!(world, "car", collections::ATOM_CAR);
    register_atom!(world, "cdr", collections::ATOM_CDR);
    register_atom!(world, "cons", collections::ATOM_CONS);
    register_atom!(world, "append", collections::ATOM_APPEND);
    register_atom!(world, "map", collections::ATOM_MAP);
    register_atom!(world, "core/str+", collections::ATOM_CORE_STR_PLUS);
    register_atom!(world, "core/map", collections::ATOM_CORE_MAP);
}

fn register_execution_atoms(world: &mut World) {
    register_atom!(world, "do", execution::ATOM_DO);
    register_atom!(world, "error", execution::ATOM_ERROR);
    register_atom!(world, "apply", execution::ATOM_APPLY);
    register_atom!(world, "for-each", execution::ATOM_FOR_EACH);
}

fn register_external_atoms(world: &mut World) {
    register_atom!(world, "print", external::ATOM_PRINT);
    register_atom!(world, "println", external::ATOM_PRINTLN);
    register_atom!(world, "output", external::ATOM_OUTPUT);
    register_atom!(world, "rand", external::ATOM_RAND);
}

fn register_string_atoms(world: &mut World) {
    register_atom!(world, "str", collections::ATOM_STR);
    register_atom!(world, "str+", collections::ATOM_STR_PLUS);
}

fn register_special_forms(world: &mut World) {
    register_atom!(world, "lambda", special_forms::ATOM_LAMBDA);
    register_atom!(world, "let", special_forms::ATOM_LET);
    register_atom!(world, "if", special_forms::ATOM_IF);
    register_atom!(world, "cond", special_forms::ATOM_COND);
    register_atom!(world, "and", special_forms::ATOM_AND);
    register_atom!(world, "or", special_forms::ATOM_OR);
    register_atom!(world, "define", special_forms::ATOM_DEFINE);
}
