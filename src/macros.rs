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
//!
//! ## Variadic Macro Forwarding
//!
//! The macro expander supports Lisp/Scheme-style variadic macro forwarding:
//! - Variadic parameters (e.g., `...args`) in call position splice their bound arguments
//! - Implemented in `substitute_template` with explicit spread (`Expr::Spread`) support
//! - Matches Scheme/Lisp semantics for idiomatic user-facing macros
//!
//! Example:
//! ```sutra
//! (define (str+ ...args)
//!   (core/str+ ...args))
//! (str+ "a" "b" "c") => (core/str+ "a" "b" "c")
//! ```
//!
//! ## Modular Architecture
//!
//! - **`expander`**: Core expansion engine and template substitution
//! - **`loader`**: Macro definition parsing, file loading, and standard macros
//! - **`std`**: Path canonicalization and standard library macros

// Standard library imports
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

// External crate imports
use miette::NamedSource;
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::{
    syntax::ParamList, errors::ErrorKind, errors::ErrorReporting, validation::ValidationContext,
};

use crate::errors::{self, DiagnosticInfo, FileContext, SourceContext, SourceInfo, SutraError};
use miette::SourceSpan;

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod expander;
mod loader;
pub mod std_macros;

// ============================================================================
// PUBLIC API ENTRY POINTS
// ============================================================================

/// Creates a new macro environment with empty registries.
///
/// This is a convenience function that creates a `MacroExpansionContext` with empty
/// user and core macro registries and no expansion trace.
pub fn create_macro_env(source: Arc<miette::NamedSource<String>>) -> MacroExpansionContext {
    MacroExpansionContext::new(source)
}

/// Creates a new macro registry.
///
/// This is a convenience function that creates an empty `MacroRegistry`.
pub fn create_macro_registry() -> MacroRegistry {
    MacroRegistry::new()
}

// ============================================================================
// CORE TYPES AND CONSTANTS
// ============================================================================

/// Maximum recursion depth for macro expansion to prevent infinite loops.
pub const MAX_MACRO_RECURSION_DEPTH: usize = 128;

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Type alias for macro expansion results
pub type MacroExpansionResult = Result<AstNode, SutraError>;

/// Type alias for macro storage and lookup
pub type MacroMap = HashMap<String, MacroDefinition>;

/// Type alias for macro lookup results with provenance
pub type MacroLookupResult<'a> = Option<(MacroOrigin, &'a MacroDefinition)>;

/// Type alias for macro registration results
pub type MacroRegistrationResult = Result<Option<MacroDefinition>, SutraError>;

/// Type alias for macro expansion trace
pub type MacroTrace = Vec<MacroExpansionStep>;

/// Unified context for macro validation operations.
///
/// Consolidates validation logic that was previously duplicated across
/// engine.rs, cli.rs, and runtime/world.rs.
#[derive(Debug)]
pub struct MacroValidationContext {
    /// Whether to check for duplicates
    pub check_duplicates: bool,
    /// Whether to allow overwrites
    pub allow_overwrites: bool,
    /// Error message template for duplicates
    pub duplicate_error_template: String,
    /// Source context for error reporting
    pub source_context: Option<Arc<NamedSource<String>>>,
}

impl Default for MacroValidationContext {
    fn default() -> Self {
        Self {
            check_duplicates: true,
            allow_overwrites: false,
            duplicate_error_template: "duplicate macro name '{}'".to_string(),
            source_context: None,
        }
    }
}

impl MacroValidationContext {
    /// Creates a context for user macro validation.
    pub fn for_user_macros() -> Self {
        Self {
            check_duplicates: true,
            allow_overwrites: false,
            duplicate_error_template: "duplicate macro name '{}'".to_string(),
            source_context: None,
        }
    }

    /// Creates a context for standard library validation.
    pub fn for_standard_library() -> Self {
        Self {
            check_duplicates: true,
            allow_overwrites: false,
            duplicate_error_template: "duplicate macro name '{}'".to_string(),
            source_context: None,
        }
    }

    /// Validates and inserts a macro into the target map.
    pub fn validate_and_insert(
        &self,
        name: String,
        definition: MacroDefinition,
        target: &mut HashMap<String, MacroDefinition>,
    ) -> Result<(), SutraError> {
        // Step 1: Check for duplicates if validation is enabled
        if self.check_duplicates && target.contains_key(&name) {
            return Err(self.report(
                ErrorKind::DuplicateDefinition {
                    symbol: name,
                    original_location: errors::unspanned(),
                },
                errors::unspanned(),
            ));
        }

        // Step 4: Insert the macro
        target.insert(name, definition);
        Ok(())
    }

    /// Validates and inserts multiple macros.
    pub fn validate_and_insert_many(
        &self,
        macros: Vec<(String, MacroTemplate)>,
        target: &mut HashMap<String, MacroDefinition>,
    ) -> Result<(), SutraError> {
        for (name, template) in macros {
            self.validate_and_insert(name, MacroDefinition::Template(template), target)?;
        }
        Ok(())
    }
}

impl ErrorReporting for MacroValidationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        let source = self
            .source_context
            .clone()
            .unwrap_or_else(|| Arc::new(NamedSource::new("macro_validation", "".to_string())));

        SutraError {
            kind: kind.clone(),
            source_info: SourceInfo {
                source,
                primary_span: span,
                file_context: FileContext::Validation {
                    phase: "macro registration".into(),
                },
            },
            diagnostic_info: DiagnosticInfo {
                help: self.generate_validation_help(&kind),
                related_spans: Vec::new(),
                error_code: format!("sutra::validation::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}

impl MacroValidationContext {
    /// Generate context-appropriate help for validation errors
    fn generate_validation_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                Some(format!("The macro '{}' is already defined. Use a different name or check for conflicting definitions.", symbol))
            }
            _ => None,
        }
    }
}

/// Configuration for macro registration behavior.
///
/// Controls validation, error handling, and overwrite behavior during registration.
#[derive(Debug, Clone)]
pub struct MacroRegistrationConfig {
    /// Whether to allow overwriting existing macros
    pub allow_overwrite: bool,
    /// Whether to validate macro names
    pub validate_name: bool,
    /// Whether to check for duplicates
    pub check_duplicates: bool,
}

impl Default for MacroRegistrationConfig {
    fn default() -> Self {
        Self {
            allow_overwrite: true,
            validate_name: true,
            check_duplicates: false,
        }
    }
}

impl MacroRegistrationConfig {
    /// Creates a configuration that prevents overwrites and validates names.
    pub fn strict() -> Self {
        Self {
            allow_overwrite: false,
            validate_name: true,
            check_duplicates: true,
        }
    }

    /// Creates a configuration that allows overwrites but validates names.
    pub fn permissive() -> Self {
        Self {
            allow_overwrite: true,
            validate_name: true,
            check_duplicates: false,
        }
    }
}

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
        // Validate no duplicate parameters
        let mut seen = HashSet::new();
        let sc = SourceContext::fallback("MacroTemplate::new");
        let context = ValidationContext::new(sc, "parameter validation".to_string());

        for name in &params.required {
            if !seen.insert(name) {
                return Err(context.report(
                    ErrorKind::DuplicateDefinition {
                        symbol: name.clone(),
                        original_location: errors::unspanned(),
                    },
                    errors::unspanned(),
                ));
            }
        }

        if let Some(var) = &params.rest {
            if !seen.insert(var) {
                return Err(context.report(
                    ErrorKind::DuplicateDefinition {
                        symbol: var.clone(),
                        original_location: errors::unspanned(),
                    },
                    errors::unspanned(),
                ));
            }
        }

        Ok(MacroTemplate { params, body })
    }
}

/// A macro definition, either a native function or a template.
///
/// Note: Only `Template` variants are serializable, as function pointers
/// cannot be serialized. The registry handles this transparently.
#[derive(Debug, Clone)]
pub enum MacroDefinition {
    /// A native Rust function macro
    Fn(MacroFunction),
    /// A declarative template macro
    Template(MacroTemplate),
}

/// Origin of a macro expansion step: user or core registry.
///
/// Used for debugging and tracing macro expansions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroOrigin {
    /// User-defined macro
    User,
    /// Built-in system macro
    Core,
}

/// A single macro expansion step, for traceability.
///
/// Records the macro name, provenance, and input/output ASTs for debugging.
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

/// Macro expansion environment: holds user/core registries and the trace.
///
/// Manages the complete macro expansion context with separate namespaces
/// for user and core macros, plus expansion trace recording.
#[derive(Debug, Clone)]
pub struct MacroExpansionContext {
    /// User-defined macros loaded from files
    pub user_macros: MacroMap,
    /// Built-in system macros
    pub core_macros: MacroMap,
    /// Trace of macro expansion steps
    pub trace: MacroTrace,
    /// The source code being processed, for error reporting.
    pub source: Arc<NamedSource<String>>,
}

impl Default for MacroExpansionContext {
    fn default() -> Self {
        Self {
            user_macros: HashMap::new(),
            core_macros: HashMap::new(),
            trace: Vec::new(),
            source: SourceContext::fallback("macro expansion").to_named_source(),
        }
    }
}

impl MacroExpansionContext {
    /// Creates a new, empty macro environment.
    pub fn new(source: Arc<NamedSource<String>>) -> Self {
        Self {
            user_macros: HashMap::new(),
            core_macros: HashMap::new(),
            trace: Vec::new(),
            source,
        }
    }

    /// Adds a user-defined macro to the environment.
    pub fn with_user_macro(mut self, name: String, def: MacroDefinition) -> Self {
        self.user_macros.insert(name, def);
        self
    }

    /// Looks up a macro by name, returning provenance and definition.
    ///
    /// Searches first in user macros, then in core macros.
    /// User macros have precedence over core macros.
    ///
    /// # Returns
    /// - `Some((provenance, macro_def))` if found
    /// - `None` if not found in either registry
    #[inline]
    pub fn lookup_macro(&self, name: &str) -> MacroLookupResult {
        // First, try to find in user macros (higher precedence)
        if let Some(def) = self.user_macros.get(name) {
            return Some((MacroOrigin::User, def));
        }
        // Then, try to find in core macros (lower precedence)
        if let Some(def) = self.core_macros.get(name) {
            return Some((MacroOrigin::Core, def));
        }
        // Not found in either registry
        None
    }

    /// Returns a reference to the macro expansion trace.
    pub fn trace(&self) -> &MacroTrace {
        &self.trace
    }

    /// Clears the macro expansion trace.
    pub fn clear_trace(&mut self) {
        self.trace.clear();
    }
}

/// Macro registry for built-in and template macros.
///
/// Stores macro definitions by name. Names are case-sensitive and should not be empty.
/// Overwriting an existing macro is silent unless using `*_or_error` methods.
///
/// # Thread Safety
/// Not thread-safe. Use a mutex or similar if sharing between threads.
///
/// # Serialization
/// Only template macros are serialized. Function macros are filtered out.
#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: MacroMap,
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates macro registration according to configuration.
    ///
    /// # Errors
    /// Returns an error if validation fails according to the config.
    fn validate_registration(
        &self,
        name: &str,
        config: &MacroRegistrationConfig,
    ) -> Result<(), SutraError> {
        let sc = SourceContext::fallback("MacroRegistry::validate_registration");
        let context = ValidationContext::new(sc, "macro validation".to_string());

        // Step 1: Validate name is not empty (if name validation is enabled)
        if config.validate_name && name.is_empty() {
            return Err(context.report(
                ErrorKind::InvalidMacro {
                    macro_name: "".to_string(),
                    reason: "Macro name cannot be empty.".to_string(),
                },
                errors::unspanned(),
            ));
        }

        // Step 2: Check for duplicate names (if duplicate checking is enabled)
        if config.check_duplicates && self.macros.contains_key(name) {
            return Err(context.report(
                ErrorKind::DuplicateDefinition {
                    symbol: name.to_string(),
                    original_location: errors::unspanned(),
                },
                errors::unspanned(),
            ));
        }

        // Step 3: Validation successful
        Ok(())
    }

    /// Registers a macro with configurable behavior.
    ///
    /// # Arguments
    /// * `name` - The macro name to register
    /// * `definition` - The macro definition to register
    /// * `config` - Registration configuration
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro was overwritten, `None` otherwise.
    ///
    /// # Errors
    /// Returns an error if validation fails according to the config.
    pub fn register_with_config(
        &mut self,
        name: &str,
        definition: MacroDefinition,
        config: MacroRegistrationConfig,
    ) -> MacroRegistrationResult {
        // Step 1: Validate registration parameters
        self.validate_registration(name, &config)?;

        // Step 2: Reject if macro exists and overwrites not allowed
        if !config.allow_overwrite && self.macros.contains_key(name) {
            let sc = SourceContext::fallback("MacroRegistry::register_with_config");
            let context = ValidationContext::new(sc, "macro registration".to_string());
            return Err(context.report(
                ErrorKind::DuplicateDefinition {
                    symbol: name.to_string(),
                    original_location: errors::unspanned(),
                },
                errors::unspanned(),
            ));
        }

        // Step 3: Insert macro and return any previous definition
        let old_macro = self.macros.insert(name.to_string(), definition);
        Ok(old_macro)
    }

    /// Registers a new function macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Errors
    /// Returns an error if the name is empty.
    pub fn register(&mut self, name: &str, func: MacroFunction) -> MacroRegistrationResult {
        self.register_with_config(
            name,
            MacroDefinition::Fn(func),
            MacroRegistrationConfig::default(),
        )
    }

    /// Registers a new function macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register` that prevents accidental overwrites.
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered or if the name is empty.
    pub fn register_or_error(&mut self, name: &str, func: MacroFunction) -> Result<(), SutraError> {
        self.register_with_config(
            name,
            MacroDefinition::Fn(func),
            MacroRegistrationConfig::strict(),
        )?;
        Ok(())
    }

    /// Registers a template macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Errors
    /// Returns an error if the name is empty.
    pub fn register_template(
        &mut self,
        name: &str,
        template: MacroTemplate,
    ) -> MacroRegistrationResult {
        self.register_with_config(
            name,
            MacroDefinition::Template(template),
            MacroRegistrationConfig::default(),
        )
    }

    /// Registers a template macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register_template` that prevents accidental overwrites.
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered or if the name is empty.
    pub fn register_template_or_error(
        &mut self,
        name: &str,
        template: MacroTemplate,
    ) -> Result<(), SutraError> {
        self.register_with_config(
            name,
            MacroDefinition::Template(template),
            MacroRegistrationConfig::strict(),
        )?;
        Ok(())
    }

    /// Unregisters a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro)` if the macro was found and removed, `None` if it didn't exist.
    pub fn unregister(&mut self, name: &str) -> Option<MacroDefinition> {
        self.macros.remove(name)
    }

    /// Looks up a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro_def)` if found, `None` if not found.
    pub fn lookup(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Checks if a macro with the given name is registered.
    ///
    /// Names are case-sensitive.
    /// Returns `true` if a macro with this name exists, `false` otherwise.
    pub fn contains(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Returns the number of registered macros.
    pub fn len(&self) -> usize {
        self.macros.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Returns an iterator over macro names.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.macros.keys()
    }

    /// Returns an iterator over macro definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MacroDefinition)> {
        self.macros.iter()
    }

    /// Clears all registered macros.
    pub fn clear(&mut self) {
        self.macros.clear();
    }
}

// ============================================================================
// CALLABLE TRAIT IMPLEMENTATION
// ============================================================================

// The `Callable` trait has been removed from the engine. Macros are dispatched
// through a separate path in the macro expander and are not part of the runtime
// evaluation call chain.

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

// Loading operations - re-exported from loader module
// Expansion operations - re-exported from expander module
pub use expander::expand_macros_recursively;
pub use loader::{
    check_arity, is_macro_definition, load_macros_from_file, parse_macro_definition,
    parse_macros_from_source,
};
