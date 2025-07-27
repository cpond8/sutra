use std::{collections::HashMap, fmt, fs, sync::Arc};

use miette::NamedSource;
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{
    atoms::StateContext,
    errors::{self, ErrorKind, ErrorReporting, SutraError},
    macros::{
        load_macros_from_file, std_macros::register_std_macros, MacroDefinition,
        MacroExpansionContext, MacroValidationContext,
    },
    runtime::source::SourceContext,
    validation::semantic::ValidationContext,
};

// Using a concrete, seedable PRNG for determinism.
type SmallRng = Xoshiro256StarStar;

// ============================================================================
// PATH: Canonical path representation for world state access
// ============================================================================

/// A canonical, type-safe representation of a path into the world state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path(pub Vec<String>);

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

// ============================================================================
// ATOM EXECUTION CONTEXT: Service container for atom evaluation
// ============================================================================

/// A container for all the services an atom might need during evaluation.
/// This struct provides a clean, type-safe way to pass dependencies to atoms.
// AtomExecutionContext has been removed. Eager atoms now receive the
// full EvaluationContext directly, simplifying the architecture and
// providing richer context for error reporting and state access.

// ============================================================================
// WORLD STATE: Data container for Sutra's world
// ============================================================================

// WorldState is now a simple container for the root Value::Map.
// All operations are mutable; cloning is disabled to enforce a single state.
#[derive(Debug)]
pub struct WorldState {
    data: Value,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            // The root of the world is always a Map.
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

    pub fn get_mut(&mut self, path: &Path) -> Option<&mut Value> {
        let mut current = &mut self.data;
        for key in &path.0 {
            let Value::Map(map) = current else {
                return None;
            };
            current = map.entry(key.clone()).or_insert(Value::Map(HashMap::new()));
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

// ============================================================================
// WORLD: Top-level container for all runtime state
// ============================================================================

// World is now mutable. Cloning is removed to enforce a single, canonical world.
#[derive(Debug)]
pub struct World {
    pub state: WorldState,
    pub prng: SmallRng,
    pub macros: MacroExpansionContext,
}

impl World {
    pub fn new() -> Self {
        let source = SourceContext::fallback("world state").to_named_source();
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_entropy(),
            macros: MacroExpansionContext::new(source),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        let source = SourceContext::fallback("world state").to_named_source();
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_seed(seed),
            macros: MacroExpansionContext::new(source),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        self.state.get(path)
    }

    // `set` now mutates the world directly.
    pub fn set(&mut self, path: &Path, val: Value) {
        self.state.set(path, val);
    }

    // `del` now mutates the world directly.
    pub fn del(&mut self, path: &Path) {
        self.state.del(path);
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
    }
}

// ============================================================================
// REGISTRY CONSTRUCTION: Canonical registry builders
// ============================================================================

// This function has been removed as the AtomRegistry is no longer used.
// Native functions are now registered directly into the World.

/// Builds and returns a fully populated macro registry with all standard macros registered.
///
/// Creates a registry containing all core/built-in macros. This is used internally
/// by `build_canonical_macro_env` and typically not called directly.
///
/// # Example
/// ```rust
/// use sutra::runtime::world::build_default_macro_registry;
/// let registry = build_default_macro_registry();
/// assert!(!registry.is_empty());
/// ```
#[inline]
pub fn build_default_macro_registry() -> MacroRegistry {
    let mut registry = MacroRegistry::new();
    register_std_macros(&mut registry);
    registry
}

/// Builds and returns the canonical macro environment (MacroExpansionContext) for the Sutra engine.
///
/// This function is the single source of truth for macro environment construction. It:
/// 1. Registers all core/built-in macros (quote, unquote, etc.)
/// 2. Loads and registers all standard macros from `src/macros/std_macros.sutra`
/// 3. Validates for duplicate macro names
/// 4. Returns a complete MacroExpansionContext ready for expansion
///
/// Must be used by all entrypoints (CLI, library, tests) to ensure consistency.
///
/// # Example
/// ```rust
/// use sutra::runtime::world::build_canonical_macro_env;
/// let env = build_canonical_macro_env().expect("Macro environment should build successfully");
/// assert!(!env.user_macros.is_empty() || !env.core_macros.is_empty());
/// ```
///
/// # Errors
/// Returns a `SutraError` if:
/// - The standard macro file (`src/macros/std_macros.sutra`) cannot be loaded or parsed
/// - Duplicate macro names are found in the standard macro library
///
/// # Safety
/// This function is pure and has no side effects. All state is explicit.
pub fn build_canonical_macro_env() -> Result<MacroExpansionContext, SutraError> {
    let core_macros = build_core_macro_registry();
    let user_macros = load_and_process_user_macros("src/macros/std_macros.sutra")?;
    let source = Arc::new(NamedSource::new(
        "std_macros.sutra",
        fs::read_to_string("src/macros/std_macros.sutra").unwrap_or_default(),
    ));

    Ok(MacroExpansionContext {
        user_macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
        source,
    })
}

// ============================================================================
// INTERNAL REGISTRY HELPERS
// ============================================================================

/// Builds and returns a macro registry with all core/built-in macros registered.
///
/// Handles the registration of fundamental macros like `quote`, `unquote`, etc.
/// Used internally by `build_canonical_macro_env`.
fn build_core_macro_registry() -> MacroRegistry {
    let mut core_registry = MacroRegistry::new();
    register_std_macros(&mut core_registry);
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    {
        // Register test-only macros here if/when they exist
        // e.g., macros::test::register_test_macros(&mut core_registry);
    }
    core_registry
}

/// Loads macros from the specified file and processes them into a user macro map.
///
/// Performs the following operations:
/// 1. Loads macro definitions from the given file path
/// 2. Validates for duplicate macro names within the file
/// 3. Converts macro templates to `MacroDefinition::Template` format
/// 4. Returns a HashMap ready for inclusion in MacroExpansionContext
///
/// Used internally by `build_canonical_macro_env`.
///
/// # Arguments
/// * `path` - File path to load macros from (typically "src/macros/std_macros.sutra")
///
/// # Errors
/// Returns a `SutraError` if:
/// - The file cannot be loaded or parsed
/// - Duplicate macro names are found within the file
fn load_and_process_user_macros(
    path: &str,
) -> Result<HashMap<String, MacroDefinition>, SutraError> {
    // Load macros from file with error logging
    let macros = load_macros_from_file(path)?;

    // Process loaded macros with duplicate checking
    let mut user_macros = HashMap::new();
    let mut ctx = MacroValidationContext::for_standard_library();
    ctx.source_context = Some(Arc::new(NamedSource::new(path.to_string(), String::new())));

    ctx.validate_and_insert_many(macros, &mut user_macros)?;
    Ok(user_macros)
}

// ============================================================================
// MUTABLE HELPERS: set_recursive_mut, del_recursive_mut
// ============================================================================

/// Recursive helper for mutable `set`.
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

/// Recursive helper for mutable `del`.
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

// ============================================================================
// STATE CONTEXT IMPLEMENTATION
// ============================================================================

impl StateContext for WorldState {
    fn get(&self, path: &Path) -> Option<&Value> {
        self.get(path)
    }

    // `set` now correctly uses the mutable implementation.
    fn set(&mut self, path: &Path, value: Value) {
        set_recursive_mut(&mut self.data, &path.0, value);
    }

    // `del` now correctly uses the mutable implementation.
    fn del(&mut self, path: &Path) {
        del_recursive_mut(&mut self.data, &path.0);
    }

    fn exists(&self, path: &Path) -> bool {
        self.get(path).is_some()
    }
}
