//!
//! Provides a single, canonical function to construct a fully populated atom and macro registry
//! for both production and test use. This eliminates duplication and ensures all code paths
//! share the same registration logic.
//!
//! ## Usage Workflow
//! ```rust
//! use sutra::runtime::registry::{build_default_atom_registry, build_canonical_macro_env};
//! // 1. Build atom registry (used for evaluation)
//! let atoms = build_default_atom_registry();
//! // 2. Build macro environment (used for expansion)
//! let macros = build_canonical_macro_env().expect("Macros should load");
//! // 3. Use registries for parsing, validation, and evaluation
//! assert!(atoms.is_empty() == false); // Atoms registry should not be empty
//! assert!(macros.core_macros.is_empty() == false); // Core macros should not be empty
//! ```
//!
//! ## Registry Invariant
//! The atom registry is a single source of truth. It must be constructed once at the entrypoint
//! and passed by reference to all validation and evaluation code. Never construct a local/hidden
//! registry. See validate.rs and atom.rs for enforcement.
//!

use crate::atoms::{self, AtomRegistry};
use crate::macros::{self, MacroRegistry};
use crate::macros::{load_macros_from_file, MacroDef, MacroEnv};
use crate::SutraError;
use crate::err_ctx;
use std::collections::HashMap;

// ============================================================================
// Public API Implementation
// ============================================================================

/// Builds and returns a fully populated atom registry with all standard atoms registered.
///
/// This function creates the canonical atom registry used by the Sutra engine. It includes
/// all standard atoms and conditionally includes test atoms in debug/test builds.
///
/// # Example
/// ```rust
/// use sutra::runtime::registry::build_default_atom_registry;
/// let registry = build_default_atom_registry();
/// assert!(!registry.is_empty());
/// ```
#[inline]
pub fn build_default_atom_registry() -> AtomRegistry {
    let mut registry = AtomRegistry::new();
    atoms::register_all_atoms(&mut registry);
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    {
        // Register test atoms only in debug/test builds
        atoms::test::register_test_atoms(&mut registry);
    }
    registry
}

/// Builds and returns a fully populated macro registry with all standard macros registered.
///
/// Creates a registry containing all core/built-in macros. This is used internally
/// by `build_canonical_macro_env` and typically not called directly.
///
/// # Example
/// ```rust
/// use sutra::runtime::registry::build_default_macro_registry;
/// let registry = build_default_macro_registry();
/// assert!(!registry.is_empty());
/// ```
#[inline]
pub fn build_default_macro_registry() -> MacroRegistry {
    let mut registry = MacroRegistry::new();
    macros::std::register_std_macros(&mut registry);
    registry
}

/// Builds and returns the canonical macro environment (MacroEnv) for the Sutra engine.
///
/// This function is the single source of truth for macro environment construction. It:
/// 1. Registers all core/built-in macros (quote, unquote, etc.)
/// 2. Loads and registers all standard macros from `src/macros/macros.sutra`
/// 3. Validates for duplicate macro names
/// 4. Returns a complete MacroEnv ready for expansion
///
/// Must be used by all entrypoints (CLI, library, tests) to ensure consistency.
///
/// # Example
/// ```rust
/// use sutra::runtime::registry::build_canonical_macro_env;
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
pub fn build_canonical_macro_env() -> Result<MacroEnv, SutraError> {
    let core_macros = build_core_macro_registry();
    let user_macros = load_and_process_user_macros("src/macros/macros.sutra")?;
    let source = std::sync::Arc::new(miette::NamedSource::new(
        "macros.sutra",
        std::fs::read_to_string("src/macros/macros.sutra").unwrap_or_default(),
    ));

    Ok(MacroEnv {
        user_macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
        source,
    })
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Builds and returns a macro registry with all core/built-in macros registered.
///
/// Handles the registration of fundamental macros like `quote`, `unquote`, etc.
/// Used internally by `build_canonical_macro_env`.
fn build_core_macro_registry() -> MacroRegistry {
    let mut core_registry = MacroRegistry::new();
    macros::std::register_std_macros(&mut core_registry);
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
/// 3. Converts macro templates to `MacroDef::Template` format
/// 4. Returns a HashMap ready for inclusion in MacroEnv
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
fn load_and_process_user_macros(path: &str) -> Result<HashMap<String, MacroDef>, SutraError> {
    // Load macros from file with error logging
    let macros = load_macros_from_file(path).map_err(|e| {
        #[cfg(debug_assertions)]
        eprintln!("[sutra:build_canonical_macro_env] Failed to load standard macros: {}", e);
        e
    })?;

    // Process loaded macros with duplicate checking
    let mut user_macros = HashMap::new();
    for (name, template) in macros {
        if user_macros.contains_key(&name) {
            return Err(err_ctx!(Validation, "Duplicate macro name '{}' in standard macro library.", name));
        }
        user_macros.insert(name, MacroDef::Template(template));
    }

    Ok(user_macros)
}
