//! # Sutra Macro Expansion System
//!
//! This module is responsible for the purely syntactic transformation of the AST
//! before evaluation. Macros allow authors to create high-level abstractions
//! that expand into simpler, core expressions.
//!
//! ## Core Principles
//!
//! - **Syntactic Only**: Macros operate solely on the AST (`Expr`). They have no access
//!   to the `World` state and cannot perform any evaluation or side effects.
//! - **Pure Transformation**: Macro expansion is a pure function: `(AST) -> Result<AST, Error>`.
//! - **Inspectable**: The expansion process can be traced, allowing authors to see
//!   how their high-level forms are desugared into core language constructs.
//! - **Layered**: The macro system is a distinct pipeline stage that runs after parsing
//!   and before validation and evaluation.

use crate::ast::Expr;
use crate::error::SutraError;
use std::collections::HashMap;

// A macro function takes an expression and attempts to transform it into another expression.
pub type MacroFn = fn(&Expr) -> Result<Expr, SutraError>;

/// A registry for all known macros, both built-in and potentially user-defined.
///
/// The registry is responsible for dispatching to the correct macro function
/// and for driving the recursive expansion process.
#[derive(Default)]
pub struct MacroRegistry {
    pub macros: HashMap<String, MacroFn>,
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new macro with the given name.
    pub fn register(&mut self, name: &str, func: MacroFn) {
        self.macros.insert(name.to_string(), func);
    }

    /// Recursively expands all macros in a given expression.
    ///
    /// This is the main entry point for the macro expansion pipeline stage.
    /// It traverses the AST and applies macro transformations wherever a macro
    /// invocation is found.
    ///
    /// `depth` is used to prevent infinite recursion.
    pub fn expand_macros(&self, expr: &Expr, depth: usize) -> Result<Expr, SutraError> {
        // TODO: Implement the recursive macro expansion logic.
        // - Check if the expression is a list and its head is a known macro.
        // - If so, call the macro function and recurse on the result.
        // - If not, recurse on the children of the expression.
        // - Handle recursion depth limiting.
        Ok(expr.clone()) // Placeholder
    }

    /// Provides a step-by-step trace of the macro expansion process.
    ///
    /// This is a powerful debugging tool for authors to understand how their
    /// code is being transformed. It returns a vector of expressions, where
    /// each element is the result of a single expansion step.
    pub fn macroexpand_trace(&self, expr: &Expr) -> Vec<Expr> {
        // TODO: Implement macro expansion tracing.
        // This will be similar to `expand_macros` but will collect intermediate
        // expansion steps.
        vec![expr.clone()] // Placeholder
    }
}
