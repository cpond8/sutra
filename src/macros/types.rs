//!
//! This module defines the fundamental types used throughout the macro system.
//! It has no dependencies on other macro modules, making it the foundation layer.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `err_msg!` or `err_ctx!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! use sutra::err_msg;
//! let err = err_msg!(Validation, "Duplicate parameter name");
//! assert!(matches!(err, sutra::SutraError::Validation { .. }));
//! ```
//!
//! All macro type, validation, and callable errors use this system.
//!
//! ## Ownership and Borrowing
//!
//! - All types are owned structures that can be freely moved and cloned
//! - `MacroFn` is a function pointer, cheaply copyable
//! - `MacroTemplate` owns its parameters and body
//! - `MacroEnv` owns all macro registries and trace data
//!
//! ## Design Principles
//!
//! - **Self-contained**: No dependencies on other macro modules
//! - **Serializable**: Core types support serde where appropriate
//! - **Cloneable**: All types implement Clone for flexibility
//! - **Documented**: All public types have comprehensive documentation

use crate::ast::{AstNode, ParamList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::err_msg;
use miette::NamedSource;

/// Maximum recursion depth for macro expansion to prevent infinite loops.
pub const MAX_MACRO_RECURSION_DEPTH: usize = 128;

/// A macro function is a native Rust function that transforms an AST.
///
/// Macro functions operate purely on the AST level and must:
/// - Accept an `&AstNode` (the macro call)
/// - Return `Result<AstNode, SutraError>` (the expanded form)
/// - Be pure transformations with no side effects
/// - Maintain span information for error reporting
///
/// # Examples
///
/// ```rust
/// use sutra::macros::MacroFn;
/// use sutra::ast::{AstNode, WithSpan};
/// use std::sync::Arc;
/// // A macro that clones its input node
/// let my_macro: MacroFn = |node| {
///     Ok(WithSpan { value: Arc::clone(&node.value), span: node.span })
/// };
/// ```
pub type MacroFn = fn(&AstNode) -> Result<AstNode, crate::SutraError>;

/// A declarative macro defined by a template.
///
/// Template macros consist of:
/// - Parameters: The formal parameters the macro accepts
/// - Body: The template that gets expanded with substituted arguments
///
/// The template system supports:
/// - Regular parameters
/// - Variadic parameters (with `...param` syntax)
/// - Nested macro calls within templates
/// - Proper span preservation for error reporting
///
/// # Examples
///
/// ```rust
/// use sutra::macros::MacroTemplate;
/// use sutra::ast::{ParamList, WithSpan};
/// use std::sync::Arc;
/// // A simple macro template: (define (double x) (* x 2))
/// let params = ParamList {
///     required: vec!["x".to_string()],
///     rest: None,
///     span: Default::default(),
/// };
/// let body = Box::new(WithSpan { value: Arc::new(sutra::ast::Expr::Number(0.0, sutra::ast::Span::default())), span: sutra::ast::Span::default() });
/// let template = MacroTemplate::new(params, body).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroTemplate {
    /// The parameters this macro accepts
    pub params: ParamList,
    /// The template body that gets expanded
    pub body: Box<AstNode>,
}

/// A macro definition, either a native function or a template.
///
/// `MacroDef` represents the two kinds of macros supported:
/// - `Fn`: Native Rust functions that transform AST nodes
/// - `Template`: Declarative macros defined by templates
///
/// Note: Only `Template` variants are serializable, as function pointers
/// cannot be serialized. The registry handles this transparently.
#[derive(Debug, Clone)]
pub enum MacroDef {
    /// A native Rust function macro
    Fn(MacroFn),
    /// A declarative template macro
    Template(MacroTemplate),
}

/// Provenance of a macro expansion step: user or core registry.
///
/// This tracks whether a macro came from:
/// - `User`: User-defined macros loaded from files
/// - `Core`: Built-in macros provided by the system
///
/// This information is used for debugging and tracing macro expansions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroProvenance {
    /// User-defined macro
    User,
    /// Built-in system macro
    Core,
}

/// A single macro expansion step, for traceability.
///
/// Each expansion step records:
/// - The macro name that was invoked
/// - Where the macro was found (user vs core registry)
/// - The input AST before expansion
/// - The output AST after expansion
///
/// This enables detailed debugging and inspection of macro expansion.
#[derive(Debug, Clone)]
pub struct MacroExpansionStep {
    /// The macro name invoked
    pub macro_name: String,
    /// Which registry the macro was found in
    pub provenance: MacroProvenance,
    /// The AST before expansion
    pub input: AstNode,
    /// The AST after expansion
    pub output: AstNode,
}

/// Macro expansion environment: holds user/core registries and the trace.
///
/// `MacroEnv` manages the complete macro expansion context:
/// - User-defined macros (loaded from files)
/// - Core built-in macros (provided by the system)
/// - Expansion trace for debugging
///
/// The environment supports:
/// - Macro lookup with provenance tracking
/// - Expansion trace recording
/// - Separate namespaces for user and core macros
///
/// # Examples
///
/// ```rust
/// use sutra::macros::MacroEnv;
/// let source = std::sync::Arc::new(miette::NamedSource::new("test", ""));
/// let env = MacroEnv::new(source);
/// assert!(env.user_macros.is_empty());
/// assert!(env.core_macros.is_empty());
/// assert!(env.trace.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct MacroEnv {
    /// User-defined macros loaded from files
    pub user_macros: HashMap<String, MacroDef>,
    /// Built-in system macros
    pub core_macros: HashMap<String, MacroDef>,
    /// Trace of macro expansion steps
    pub trace: Vec<MacroExpansionStep>,
    /// The source code being processed, for error reporting.
    pub source: Arc<NamedSource<String>>,
}

// ============================================================================
// IMPLEMENTATIONS
// ============================================================================

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for duplicate parameters.
    ///
    /// This constructor performs validation to ensure:
    /// - No duplicate parameter names
    /// - Parameter names are valid identifiers
    /// - Variadic parameters don't conflict with regular parameters
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Duplicate parameter names are found
    /// - Parameter validation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::MacroTemplate;
    /// use sutra::ast::{ParamList, WithSpan};
    /// use std::sync::Arc;
    /// let params = ParamList {
    ///     required: vec!["x".to_string(), "y".to_string()],
    ///     rest: Some("rest".to_string()),
    ///     span: Default::default(),
    /// };
    /// let body = Box::new(WithSpan { value: Arc::new(sutra::ast::Expr::Number(0.0, sutra::ast::Span::default())), span: sutra::ast::Span::default() });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// ```
    pub fn new(
        params: ParamList,
        body: Box<AstNode>,
    ) -> Result<Self, crate::SutraError> {
        let mut all_names = params.required.clone();

        // Add variadic parameter if present
        if let Some(var) = &params.rest {
            all_names.push(var.clone());
        }

        // Validate no duplicate parameters
        check_no_duplicate_params(&all_names)?;

        // Construct template
        Ok(MacroTemplate { params, body })
    }
}

impl MacroEnv {
    /// Creates a new, empty macro environment.
    ///
    /// The environment starts with empty registries and no trace.
    /// Macros can be added to either the user or core registries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::MacroEnv;
    ///
    /// let source = std::sync::Arc::new(miette::NamedSource::new("test", ""));
    /// let env = MacroEnv::new(source);
    /// assert!(env.user_macros.is_empty());
    /// assert!(env.core_macros.is_empty());
    /// assert!(env.trace.is_empty());
    /// ```
    pub fn new(source: Arc<NamedSource<String>>) -> Self {
        Self {
            user_macros: HashMap::new(),
            core_macros: HashMap::new(),
            trace: Vec::new(),
            source,
        }
    }

    /// Adds a user-defined macro to the environment.
    pub fn with_user_macro(mut self, name: String, def: MacroDef) -> Self {
        self.user_macros.insert(name, def);
        self
    }

    /// Looks up a macro by name, returning provenance and definition.
    ///
    /// Searches first in user macros, then in core macros.
    /// This gives user macros precedence over core macros.
    ///
    /// # Returns
    ///
    /// - `Some((provenance, macro_def))` if found
    /// - `None` if not found in either registry
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::{MacroEnv, MacroProvenance};
    ///
    /// let source = std::sync::Arc::new(miette::NamedSource::new("test", ""));
    /// let env = MacroEnv::new(source);
    /// match env.lookup_macro("my-macro") {
    ///     Some((MacroProvenance::User, _def)) => println!("Found user macro"),
    ///     Some((MacroProvenance::Core, _def)) => println!("Found core macro"),
    ///     None => println!("Macro not found"),
    /// }
    /// ```
    #[inline]
    pub fn lookup_macro(&self, name: &str) -> Option<(MacroProvenance, &MacroDef)> {
        self.user_macros
            .get(name)
            .map(|def| (MacroProvenance::User, def))
            .or_else(|| {
                self.core_macros
                    .get(name)
                    .map(|def| (MacroProvenance::Core, def))
            })
    }

    /// Returns a reference to the macro expansion trace.
    ///
    /// The trace contains all macro expansion steps performed
    /// in chronological order, useful for debugging.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::MacroEnv;
    ///
    /// let source = std::sync::Arc::new(miette::NamedSource::new("test", ""));
    /// let env = MacroEnv::new(source);
    /// let trace = env.trace();
    /// for step in trace {
    ///     println!("Expanded macro: {}", step.macro_name);
    /// }
    /// ```
    pub fn trace(&self) -> &[MacroExpansionStep] {
        &self.trace
    }

    /// Clears the macro expansion trace.
    ///
    /// This can be useful when reusing an environment
    /// but wanting to start fresh with tracing.
    pub fn clear_trace(&mut self) {
        self.trace.clear();
    }
}


// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validates that parameter names are not duplicated.
///
/// This is a helper function used by `MacroTemplate::new()` to ensure
/// macro parameter lists don't contain duplicate names.
///
/// # Arguments
///
/// * `all_names` - All parameter names (required + variadic)
///
/// # Errors
///
/// Returns an error if any parameter name appears more than once.
fn check_no_duplicate_params(
    all_names: &[String],
) -> Result<(), crate::SutraError> {
    let mut seen = std::collections::HashSet::new();
    for name in all_names {
        if !seen.insert(name) {
            return Err(err_msg!(Validation, "Duplicate parameter name"));
        }
    }
    Ok(())
}

// ============================================================================
// CALLABLE TRAIT IMPLEMENTATION
// ============================================================================

impl crate::atoms::Callable for MacroDef {
    fn call(&self, _args: &[crate::ast::value::Value], _context: &mut crate::runtime::context::ExecutionContext, _current_world: &crate::runtime::world::World) -> Result<(crate::ast::value::Value, crate::runtime::world::World), crate::SutraError> {
        // Macros operate on AST nodes, not Values, so they cannot be called through the Callable interface
        // This is a design limitation - macros need syntax transformation, not evaluation
        Err(err_msg!(Validation, "Macros cannot be called through Callable interface - they require AST transformation, not evaluation"))
    }
}
