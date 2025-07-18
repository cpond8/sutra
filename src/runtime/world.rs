use crate::ast::value::Value;
use im::HashMap;
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use crate::atoms::{StateContext, SharedOutput};
use crate::atoms::{self, AtomRegistry};
use crate::macros::{self, MacroRegistry};
use crate::macros::{load_macros_from_file, MacroDefinition, MacroExpansionContext};
use crate::SutraError;
use crate::err_ctx;
use std::collections::HashMap as StdHashMap;
use serde::{Deserialize, Serialize};
use std::fmt;

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
pub struct AtomExecutionContext<'a> {
    pub state: &'a mut dyn StateContext,
    pub output: SharedOutput,
    pub rng: &'a mut dyn RngCore,
}

// ============================================================================
// WORLD STATE: Data container for Sutra's world
// ============================================================================

#[derive(Clone, Debug)]
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
            let Value::Map(map) = current else { return None };
            let value = map.get(key.as_str())?;
            current = value;
        }
        Some(current)
    }

    pub fn set(&self, path: &Path, val: Value) -> Self {
        if path.0.is_empty() {
            return self.clone();
        }
        let new_data = set_recursive(&self.data, &path.0, val);
        Self { data: new_data }
    }

    pub fn del(&self, path: &Path) -> Self {
        if path.0.is_empty() {
            return self.clone();
        }
        let new_data = del_recursive(&self.data, &path.0);
        Self { data: new_data }
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

#[derive(Clone, Debug)]
pub struct World {
    pub state: WorldState,
    pub prng: SmallRng,
    pub macros: crate::macros::MacroExpansionContext,
}

impl World {
    pub fn new() -> Self {
        let source = std::sync::Arc::new(miette::NamedSource::new("empty", "".to_string()));
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_entropy(),
            macros: macros::MacroExpansionContext::new(source),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        let source = std::sync::Arc::new(miette::NamedSource::new("empty", "".to_string()));
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_seed(seed),
            macros: macros::MacroExpansionContext::new(source),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        self.state.get(path)
    }

    pub fn set(&self, path: &Path, val: Value) -> Self {
        Self {
            state: self.state.set(path, val),
            ..self.clone()
        }
    }

    pub fn del(&self, path: &Path) -> Self {
        Self {
            state: self.state.del(path),
            ..self.clone()
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
    }

    pub fn with_macros(self, macros: crate::macros::MacroExpansionContext) -> Self {
        Self { macros, ..self }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REGISTRY CONSTRUCTION: Canonical registry builders
// ============================================================================

/// Builds and returns a fully populated atom registry with all standard atoms registered.
///
/// This function creates the canonical atom registry used by the Sutra engine. It includes
/// all standard atoms and conditionally includes test atoms in debug/test builds.
///
/// # Example
/// ```rust
/// use sutra::runtime::world::build_default_atom_registry;
/// let registry = build_default_atom_registry();
/// assert!(!registry.is_empty());
/// ```
#[inline]
pub fn build_default_atom_registry() -> AtomRegistry {
    let mut registry = AtomRegistry::new();
    atoms::register_all_atoms(&mut registry);
    registry
}

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
    macros::std_macros::register_std_macros(&mut registry);
    registry
}

/// Builds and returns the canonical macro environment (MacroExpansionContext) for the Sutra engine.
///
/// This function is the single source of truth for macro environment construction. It:
/// 1. Registers all core/built-in macros (quote, unquote, etc.)
/// 2. Loads and registers all standard macros from `src/macros/macros.sutra`
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
/// - The standard macro file (`src/macros/macros.sutra`) cannot be loaded or parsed
/// - Duplicate macro names are found in the standard macro library
///
/// # Safety
/// This function is pure and has no side effects. All state is explicit.
pub fn build_canonical_macro_env() -> Result<MacroExpansionContext, SutraError> {
    let core_macros = build_core_macro_registry();
    let user_macros = load_and_process_user_macros("src/macros/macros.sutra")?;
    let source = std::sync::Arc::new(miette::NamedSource::new(
        "macros.sutra",
        std::fs::read_to_string("src/macros/macros.sutra").unwrap_or_default(),
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
    macros::std_macros::register_std_macros(&mut core_registry);
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
/// * `path` - File path to load macros from (typically "src/macros/macros.sutra")
///
/// # Errors
/// Returns a `SutraError` if:
/// - The file cannot be loaded or parsed
/// - Duplicate macro names are found within the file
fn load_and_process_user_macros(path: &str) -> Result<StdHashMap<String, MacroDefinition>, SutraError> {
    // Load macros from file with error logging
    let macros = load_macros_from_file(path).map_err(|e| {
        #[cfg(debug_assertions)]
        eprintln!("[sutra:build_canonical_macro_env] Failed to load standard macros: {e}");
        e
    })?;

    // Process loaded macros with duplicate checking
    let mut user_macros = StdHashMap::new();
    for (name, template) in macros {
        if user_macros.contains_key(&name) {
            let src_arc = crate::diagnostics::to_error_source(&name);
            return Err(err_ctx!(Validation, format!("Duplicate macro name '{}' in standard macro library.", name), &src_arc, crate::ast::Span::default(), "Duplicate macro name in standard macro library."));
        }
        user_macros.insert(name, MacroDefinition::Template(template));
    }

    Ok(user_macros)
}

// ============================================================================
// IMMUTABLE HELPERS: set_recursive, del_recursive
// ============================================================================

// Recursive helper for immutable `set`.
fn set_recursive(current: &Value, path_segments: &[String], val: Value) -> Value {
    let Some(key) = path_segments.first() else {
        return current.clone();
    };

    let remaining_segments = &path_segments[1..];
    let mut map = match current {
        Value::Map(m) => m.clone(),
        _ => HashMap::new(),
    };

    if remaining_segments.is_empty() {
        map.insert(key.clone(), val);
    } else {
        let child = map.get(key).unwrap_or(&Value::Nil);
        let new_child = set_recursive(child, remaining_segments, val);
        map.insert(key.clone(), new_child);
    }

    Value::Map(map)
}

// Recursive helper for immutable `del`.
fn del_recursive(current: &Value, path_segments: &[String]) -> Value {
    let Some(key) = path_segments.first() else {
        return current.clone();
    };

    let Value::Map(current_map) = current else {
        return current.clone();
    };

    let mut map = current_map.clone();

    if path_segments.len() == 1 {
        map.remove(key);
    } else if let Some(child) = map.get(key) {
        let new_child = del_recursive(child, &path_segments[1..]);
        if let Value::Map(child_map) = &new_child {
            if child_map.is_empty() {
                map.remove(key);
            } else {
                map.insert(key.clone(), new_child);
            }
        } else {
            map.insert(key.clone(), new_child);
        }
    }

    Value::Map(map)
}

// ============================================================================
// STATE CONTEXT IMPLEMENTATION
// ============================================================================

impl crate::atoms::StateContext for WorldState {
    fn get(&self, path: &Path) -> Option<&crate::ast::value::Value> {
        self.get(path)
    }

    fn set(&mut self, path: &Path, value: crate::ast::value::Value) {
        if path.0.is_empty() {
            return;
        }
        self.data = set_recursive(&self.data, &path.0, value);
    }

    fn del(&mut self, path: &Path) {
        if path.0.is_empty() {
            return;
        }
        self.data = del_recursive(&self.data, &path.0);
    }

    fn exists(&self, path: &Path) -> bool {
        self.get(path).is_some()
    }
}
