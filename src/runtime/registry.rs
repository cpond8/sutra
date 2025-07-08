//! # Sutra Engine: Canonical Registry Builder
//!
//! Provides a single, canonical function to construct a fully populated atom and macro registry
//! for both production and test use. This eliminates duplication and ensures all code paths
//! share the same registration logic.
//!
//! Registry Invariant: The atom registry is a single source of truth. It must be constructed once at the entrypoint and passed by reference to all validation and evaluation code. Never construct a local/hidden registry. See validate.rs and atom.rs for enforcement.

use crate::atoms::{self, AtomRegistry};
use crate::macros::{self, MacroRegistry};

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
