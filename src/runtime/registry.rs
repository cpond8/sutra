//! # Sutra Engine: Canonical Registry Builder
//!
//! Provides a single, canonical function to construct a fully populated atom and macro registry
//! for both production and test use. This eliminates duplication and ensures all code paths
//! share the same registration logic.
//!
//! Registry Invariant: The atom registry is a single source of truth. It must be constructed once at the entrypoint and passed by reference to all validation and evaluation code. Never construct a local/hidden registry. See validate.rs and atom.rs for enforcement.

use crate::atoms::{self, AtomRegistry};
use crate::macros::{self, MacroRegistry};
use crate::macros::{load_macros_from_file, MacroDef, MacroEnv};
use crate::syntax::error::{macro_error, SutraError};
use std::collections::HashMap;

/// Builds and returns a fully populated atom registry with all standard atoms registered.
///
/// # Example
/// ```
/// use sutra::runtime::registry::build_default_atom_registry;
/// let registry = build_default_atom_registry();
/// ```
#[inline]
pub fn build_default_atom_registry() -> AtomRegistry {
    let mut registry = AtomRegistry::new();
    atoms::std::register_std_atoms(&mut registry);
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    {
        // Register test atoms only in debug/test builds
        atoms::test::register_test_atoms(&mut registry);
    }
    registry
}

/// Builds and returns a fully populated macro registry with all standard macros registered.
///
/// # Example
/// ```
/// use sutra::runtime::registry::build_default_macro_registry;
/// let registry = build_default_macro_registry();
/// ```
#[inline]
pub fn build_default_macro_registry() -> MacroRegistry {
    let mut registry = MacroRegistry::new();
    macros::std::register_std_macros(&mut registry);
    registry
}

/// Builds and returns the canonical macro environment (MacroEnv) for the Sutra engine.
///
/// This function registers all core/built-in macros and loads all standard macros from
/// `src/macros/macros.sutra`. It is the single source of truth for macro environment
/// construction and must be used by all entrypoints (CLI, library, tests).
///
/// # Example
/// ```
/// use sutra::runtime::registry::build_canonical_macro_env;
/// let env = build_canonical_macro_env().expect("Macro environment should build successfully");
/// assert!(env.user_macros.contains_key("str+"));
/// ```
///
/// # Errors
/// Returns a `SutraError` if the standard macro file cannot be loaded or parsed.
///
/// # Safety
/// This function is pure and has no side effects. All state is explicit.
pub fn build_canonical_macro_env() -> Result<MacroEnv, SutraError> {
    // 1. Register all core/built-in macros
    let mut core_registry = MacroRegistry::new();
    macros::std::register_std_macros(&mut core_registry);
    #[cfg(any(test, feature = "test-atom", debug_assertions))]
    {
        // Register test-only macros here if/when they exist
        // e.g., macros::test::register_test_macros(&mut core_registry);
    }

    // 2. Load and register all standard macros from src/macros/macros.sutra
    let user_macros_path = "src/macros/macros.sutra";
    let mut user_macros = HashMap::new();
    match load_macros_from_file(user_macros_path) {
        Ok(macros) => {
            for (name, template) in macros {
                if user_macros.contains_key(&name) {
                    return Err(macro_error(
                        format!("Duplicate macro name '{}' in standard macro library.", name),
                        None,
                    ));
                }
                user_macros.insert(name, MacroDef::Template(template));
            }
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[sutra:build_canonical_macro_env] Failed to load standard macros: {}",
                    e
                );
            }
            return Err(e);
        }
    }

    // 3. Construct MacroEnv
    let env = MacroEnv {
        user_macros,
        core_macros: core_registry.macros,
        trace: Vec::new(),
    };
    Ok(env)
}
