//! # Sutra Macro Expansion System
//!
//! This module handles purely syntactic transformation of the AST before evaluation.
//! Macros allow authors to create high-level abstractions that expand into simpler,
//! core expressions.
//!
//! ## Core Principles
//!
//! - **Syntactic Only**: Macros operate solely on the AST (`AstNode`) with no access
//!   to `World` state or side effects
//! - **Pure Transformation**: Expansion is a pure function: `(AstNode) -> Result<AstNode, SutraError>`
//! - **Unified Error System**: All errors use miette-native `SutraError` variants directly
//! - **Inspectable**: Expansion process can be traced for debugging
//! - **Layered**: Runs after parsing, before validation and evaluation
//!
//! **INVARIANT:** All macro system logic operates on `AstNode`. Never unwrap to bare `Expr`
//! except for internal logic, and always re-wrap with correct span. All lists are `Vec<AstNode>`.

use miette::NamedSource;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::prelude::*;
use crate::{
    errors::{ErrorKind, ErrorReporting, SourceContext, SutraError},
    syntax::ParamList,
    validation::ValidationContext,
};

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod expander;
mod loader;
pub mod std_macros;

// ============================================================================
// CORE CONSTANTS
// ============================================================================

/// Maximum recursion depth for macro expansion to prevent infinite loops.
pub const MAX_MACRO_RECURSION_DEPTH: usize = 128;

// ============================================================================
// CORE TYPES
// ============================================================================

/// A native Rust function that transforms an AST.
///
/// Macro functions must:
/// - Accept `&AstNode` (the macro call)
/// - Return `Result<AstNode, SutraError>` (the expanded form)
/// - Be pure transformations with no side effects
/// - Maintain span information for error reporting
pub type MacroFunction = fn(&AstNode, &SourceContext) -> Result<AstNode, SutraError>;

/// A declarative macro defined by a template.
///
/// Template macros consist of parameters and a body template that gets expanded
/// with substituted arguments. Supports regular parameters, variadic parameters
/// (with `...param` syntax), nested macro calls, and proper span preservation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroTemplate {
    /// The parameters this macro accepts
    pub params: ParamList,
    /// The template body that gets expanded
    pub body: Box<AstNode>,
}

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for duplicate parameters.
    ///
    /// # Errors
    /// Returns an error if duplicate parameter names are found.
    pub fn new(params: ParamList, body: Box<AstNode>) -> Result<Self, SutraError> {
        use std::collections::HashSet;

        // Validate no duplicate parameters
        let mut seen = HashSet::new();
        let sc = SourceContext::fallback("MacroTemplate::new");
        let context = ValidationContext::new(sc, "parameter validation".to_string());

        for name in &params.required {
            if !seen.insert(name) {
                return Err(context.report(
                    ErrorKind::DuplicateDefinition {
                        symbol: name.clone(),
                        original_location: crate::errors::unspanned(),
                    },
                    crate::errors::unspanned(),
                ));
            }
        }

        if let Some(var) = &params.rest {
            if !seen.insert(var) {
                return Err(context.report(
                    ErrorKind::DuplicateDefinition {
                        symbol: var.clone(),
                        original_location: crate::errors::unspanned(),
                    },
                    crate::errors::unspanned(),
                ));
            }
        }

        Ok(MacroTemplate { params, body })
    }
}

/// A macro definition, either a native function or a template.
#[derive(Debug, Clone)]
pub enum MacroDefinition {
    /// A native Rust function macro
    Fn(MacroFunction),
    /// A declarative template macro
    Template(MacroTemplate),
}

/// Origin of a macro expansion step: user or core registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroOrigin {
    /// User-defined macro
    User,
    /// Built-in system macro
    Core,
}

/// A single macro expansion step, for traceability.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroExpansionStep {
    /// The macro name invoked
    pub macro_name: String,
    /// Which registry the macro was found in
    pub provenance: MacroOrigin,
    /// The AST before expansion
    pub input: AstNode,
    /// The AST after expansion
    pub output: AstNode,
}

/// Complete macro expansion environment.
///
/// Manages the complete macro expansion context with separate namespaces
/// for user and core macros, plus expansion trace recording.
#[derive(Debug, Clone)]
pub struct MacroEnvironment {
    pub user_macros: HashMap<String, MacroDefinition>,
    pub core_macros: HashMap<String, MacroDefinition>,
    pub trace: Vec<MacroExpansionStep>,
    pub source: Arc<NamedSource<String>>,
}

impl Default for MacroEnvironment {
    fn default() -> Self {
        Self {
            user_macros: HashMap::new(),
            core_macros: HashMap::new(),
            trace: Vec::new(),
            source: SourceContext::fallback("macro expansion").to_named_source(),
        }
    }
}

impl MacroEnvironment {
    /// Creates a new, empty macro environment.
    pub fn new(source: Arc<NamedSource<String>>) -> Self {
        Self {
            user_macros: HashMap::new(),
            core_macros: HashMap::new(),
            trace: Vec::new(),
            source,
        }
    }

    /// Register a user macro, optionally allowing overwrites.
    pub fn register_user_macro(
        &mut self,
        name: String,
        definition: MacroDefinition,
        allow_overwrite: bool,
    ) -> Result<(), SutraError> {
        if !allow_overwrite && self.user_macros.contains_key(&name) {
            let sc = SourceContext::from_file("macro_registration", &name);
            let context = ValidationContext::new(sc, "macro registration".to_string());
            return Err(context.report(
                ErrorKind::DuplicateDefinition {
                    symbol: name,
                    original_location: crate::errors::unspanned(),
                },
                crate::errors::unspanned(),
            ));
        }
        self.user_macros.insert(name, definition);
        Ok(())
    }

    /// Register a core macro (typically only during initialization).
    pub fn register_core_macro(&mut self, name: String, definition: MacroDefinition) {
        self.core_macros.insert(name, definition);
    }

    /// Look up a macro by name (user macros have precedence).
    pub fn lookup_macro(&self, name: &str) -> Option<(MacroOrigin, &MacroDefinition)> {
        if let Some(def) = self.user_macros.get(name) {
            Some((MacroOrigin::User, def))
        } else if let Some(def) = self.core_macros.get(name) {
            Some((MacroOrigin::Core, def))
        } else {
            None
        }
    }

    /// Record an expansion step.
    pub fn record_expansion(&mut self, step: MacroExpansionStep) {
        self.trace.push(step);
    }

    /// Get expansion trace.
    pub fn trace(&self) -> &[MacroExpansionStep] {
        &self.trace
    }

    /// Clear expansion trace.
    pub fn clear_trace(&mut self) {
        self.trace.clear();
    }

    /// Get source reference.
    pub fn source(&self) -> &Arc<NamedSource<String>> {
        &self.source
    }
}

// ============================================================================
// TYPE ALIASES FOR COMPATIBILITY
// ============================================================================

/// Type alias for macro expansion results
pub type MacroExpansionResult = Result<AstNode, SutraError>;

/// Type alias for macro expansion context (now points to MacroEnvironment)
pub type MacroExpansionContext = MacroEnvironment;

/// Type alias for macro expansion trace
pub type MacroTrace = Vec<MacroExpansionStep>;

// ============================================================================
// PUBLIC API ENTRY POINTS
// ============================================================================

/// Creates a new macro environment with empty registries.
pub fn create_macro_env(source: Arc<NamedSource<String>>) -> MacroEnvironment {
    MacroEnvironment::new(source)
}

/// Main expansion function.
pub fn expand_macros(ast: AstNode, env: &mut MacroEnvironment) -> Result<AstNode, SutraError> {
    expander::expand_macros_recursively(ast, env)
}

/// Load macros from source code.
pub fn load_macros_from_source(source: &str, env: &mut MacroEnvironment) -> Result<(), SutraError> {
    let macros = loader::parse_macros_from_source(source)?;
    for (name, template) in macros {
        env.register_user_macro(name, MacroDefinition::Template(template), false)?;
    }
    Ok(())
}

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

pub use expander::expand_macros_recursively;
pub use loader::{
    check_arity, is_macro_definition, load_macros_from_file, parse_macro_definition,
    parse_macros_from_source,
};
