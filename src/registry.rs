//! # Sutra Engine: Canonical Registry Builder
//!
//! Provides a single, canonical function to construct a fully populated atom and macro registry
//! for both production and test use. This eliminates duplication and ensures all code paths
//! share the same registration logic.
//!
//! Registry Invariant: The atom registry is a single source of truth. It must be constructed once at the entrypoint and passed by reference to all validation and evaluation code. Never construct a local/hidden registry. See validate.rs and atom.rs for enforcement.

use crate::atom::AtomRegistry;
use crate::atoms_std;
use crate::macros::MacroRegistry;
use crate::macros_std;

/// Builds and returns a fully populated atom registry with all standard atoms registered.
///
/// # Example
/// ```
/// use sutra::registry::build_default_atom_registry;
/// let registry = build_default_atom_registry();
/// ```
#[inline]
pub fn build_default_atom_registry() -> AtomRegistry {
    let mut registry = AtomRegistry::new();
    atoms_std::register_std_atoms(&mut registry);
    #[cfg(feature = "test-atom")]
    {
        // Regression: Ensure test atoms are only registered when the 'test-atom' feature is enabled.
        // This proves the single source of truth registry is used by all pipeline stages.
        atoms_std::register_test_atoms(&mut registry);
    }
    registry
}

/// Builds and returns a fully populated macro registry with all standard macros registered.
///
/// # Example
/// ```
/// use sutra::registry::build_default_macro_registry;
/// let registry = build_default_macro_registry();
/// ```
#[inline]
pub fn build_default_macro_registry() -> MacroRegistry {
    let mut registry = MacroRegistry::new();
    macros_std::register_std_macros(&mut registry);
    registry
}
