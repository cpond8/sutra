//! # Sutra Macro Expansion System
//!
//! This module is responsible for the purely syntactic transformation of the AST
//! before evaluation. Macros allow authors to create high-level abstractions
//! that expand into simpler, core expressions.
//!
//! ## Core Principles
//!
//! - **Syntactic Only**: Macros operate solely on the AST (`AstNode`). They have no access
//!   to the `World` state and cannot perform any evaluation or side effects.
//! - **Pure Transformation**: Macro expansion is a pure function: `(AstNode) -> Result<AstNode, SutraError>`.
//! - **Unified Error System**: All errors are reported via the unified `SutraError` type, constructed with the `err_msg!` or `err_ctx!` macro. See `src/diagnostics.rs` for details and usage patterns.
//! - **Inspectable**: The expansion process can be traced, allowing authors to see
//!   how their high-level forms are desugared into core language constructs.
//! - **Layered**: The macro system is a distinct pipeline stage that runs after parsing
//!   and before validation and evaluation.
//!
//! **INVARIANT:** All macro system logic, macro functions, and recursive expansion must operate on `AstNode`. Never unwrap to a bare `Expr` except for internal logic, and always re-wrap with the correct span. All lists are `Vec<AstNode>`.
//!
//! ## Error Handling Example
//!
//! (Doctest for err_ctx! omitted due to macro system limitations.)
//!
//! See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! ## Variadic Macro Forwarding (Argument Splicing)
//!
//! As of July 2024, the macro expander fully supports canonical Lisp/Scheme-style variadic macro forwarding:
//! - When a macro definition uses a variadic parameter (e.g., ...args), and the macro body references that parameter in call position, the macro expander splices its bound arguments as individual arguments, not as a single list.
//! - This is implemented in `substitute_template`. If a symbol in call position is bound to a list (as with a variadic parameter), its elements are spliced into the parent list. Explicit spread (`Expr::Spread`) is also supported.
//! - This matches Scheme/Lisp semantics and is required for idiomatic user-facing macros. See language spec and design doc for rationale and pseudocode.
//!
//! Example:
//!   (define (str+ ...args)
//!     (core/str+ ...args))
//!   (str+ "a" "b" "c") => (core/str+ "a" "b" "c")
//!
//! See documentation below for details and edge cases.
//!
//! ## Modular Architecture
//!
//! The macro system is organized into focused modules:
//!
//! - **`types`**: Core data structures and types
//! - **`registry`**: Macro storage and lookup operations
//! - **`loader`**: Macro definition parsing and file loading
//! - **`expander`**: Core expansion engine and template substitution
//! - **`std`**: Standard library macros
//!
//! This modular design provides:
//! - **Encapsulation**: Each module owns its domain completely
//! - **Testability**: Modules can be tested in isolation
//! - **Maintainability**: Changes are isolated to appropriate modules
//! - **Token Efficiency**: Only load relevant modules for AI context

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod expander;
mod loader;
mod registry;
mod types;

pub mod std;
pub mod definition;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

// Core types - re-exported from types module
pub use types::{
    MacroDef, MacroEnv, MacroExpansionStep, MacroFn, MacroProvenance, MacroTemplate,
    MAX_MACRO_RECURSION_DEPTH,
};

// Error types - re-exported from error module

// Registry operations - re-exported from registry module
pub use registry::MacroRegistry;

// Loading operations - re-exported from loader module
pub use loader::{check_arity, load_macros_from_file, parse_macros_from_source};

// Expansion operations - re-exported from expander module
pub use expander::{bind_macro_params, expand_macros, expand_template, substitute_template};

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Creates a new macro environment with empty registries.
///
/// This is a convenience function that creates a `MacroEnv` with empty
/// user and core macro registries and no expansion trace.
///
/// # Examples
///
/// ```rust
/// use sutra::macros::create_macro_env;
///
/// let env = create_macro_env();
/// assert!(env.user_macros.is_empty());
/// assert!(env.core_macros.is_empty());
/// ```
pub fn create_macro_env() -> MacroEnv {
    MacroEnv::new()
}

/// Creates a new macro registry.
///
/// This is a convenience function that creates an empty `MacroRegistry`.
///
/// # Examples
///
/// ```rust
/// use sutra::macros::create_macro_registry;
///
/// let registry = create_macro_registry();
/// assert!(registry.is_empty());
/// ```
pub fn create_macro_registry() -> MacroRegistry {
    MacroRegistry::new()
}
