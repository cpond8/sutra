use std::{cell::RefCell, rc::Rc};

use crate::prelude::*;
use crate::{atoms::register_all_atoms, runtime::world::World};

pub mod eval;
pub mod source;
pub mod world;

// ============================================================================
// CANONICAL BUILDERS
// ============================================================================

/// Builds and returns a canonical, fully-populated world for evaluation.
///
/// This is the single, authoritative entry point for creating a `World`. It guarantees
/// that all built-in native functions ("atoms") are registered, ensuring that the
/// environment is always ready for execution. All evaluation entry points (CLI,
/// test runner, etc.) must use this function.
///
/// Under the new model, it is impossible to create a "blank" world for evaluation,
/// which was a significant source of bugs.
pub fn build_canonical_world() -> CanonicalWorld {
    let mut world = World::new();
    register_all_atoms(&mut world);
    Rc::new(RefCell::new(world))
}

// Re-exports for concise imports
pub use self::world::build_canonical_macro_env;
pub use source::SourceContext;
