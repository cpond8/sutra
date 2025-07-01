//! # Sutra Standard Macro Library
//!
//! This module defines the standard set of macros that provide the core
//! authoring experience for narrative and gameplay logic. These macros
//! are designed to be compositional and to expand into simpler expressions
//! that the atom evaluator can understand.
//!
//! ## Core Macros
//!
//! - `storylet`: The fundamental unit of content.
//! - `choice`: Represents a player decision point.
//! - `pool`: A collection of storylets for selection.
//! - `select`: Logic for choosing from a pool.
//! - And many more for predicates, mutations, and control flow.

use crate::macro_registry::MacroRegistry;

/// Registers all standard macros into the given `MacroRegistry`.
///
/// This is the single entry point for populating the engine with the
/// standard library of authoring constructs.
pub fn register_standard_macros(registry: &mut MacroRegistry) {
    // TODO: Implement and register all standard macros.
    //
    // Example registration:
    //
    // fn expand_storylet(expr: &Expr) -> Result<Expr, SutraError> { ... }
    // registry.register("storylet", expand_storylet);
    //
    // fn expand_choice(expr: &Expr) -> Result<Expr, SutraError> { ... }
    // registry.register("choice", expand_choice);
}
