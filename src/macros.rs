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
//! - **`expander`**: Core expansion engine and template substitution
//! - **`loader`**: Macro definition parsing, file loading, and standard macros
//! - **`std`**: Path canonicalization and standard library macros
//!
//! This modular design provides:
//! - **Encapsulation**: Each module owns its domain completely
//! - **Testability**: Modules can be tested in isolation
//! - **Maintainability**: Changes are isolated to appropriate modules
//! - **Token Efficiency**: Only load relevant modules for AI context

use ::std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use miette::NamedSource;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ast::{AstNode, ParamList},
    err_ctx, err_msg, to_error_source, AtomExecutionContext, Span, Value, World,
};

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod expander;
mod loader;
pub mod std_macros;

// ============================================================================
// CORE TYPES AND CONSTANTS
// ============================================================================

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
/// use std::sync::Arc;
///
/// use sutra::{
///     ast::{AstNode, Spanned},
///     macros::MacroFunction,
/// };
/// // A macro that clones its input node
/// let my_macro: MacroFunction = |node| {
///     Ok(Spanned {
///         value: Arc::clone(&node.value),
///         span: node.span,
///     })
/// };
/// ```
pub type MacroFunction = fn(&AstNode) -> Result<AstNode, crate::SutraError>;

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
/// use std::sync::Arc;
///
/// use sutra::{
///     ast::{ParamList, Spanned},
///     macros::MacroTemplate,
/// };
/// // A simple macro template: (define (double x) (* x 2))
/// let params = ParamList {
///     required: vec!["x".to_string()],
///     rest: None,
///     span: Default::default(),
/// };
/// let body = Box::new(Spanned {
///     value: Arc::new(sutra::ast::Expr::Number(0.0, sutra::ast::Span::default())),
///     span: sutra::ast::Span::default(),
/// });
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
/// `MacroDefinition` represents the two kinds of macros supported:
/// - `Fn`: Native Rust functions that transform AST nodes
/// - `Template`: Declarative macros defined by templates
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
/// This tracks whether a macro came from:
/// - `User`: User-defined macros loaded from files
/// - `Core`: Built-in macros provided by the system
///
/// This information is used for debugging and tracing macro expansions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroOrigin {
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
    pub provenance: MacroOrigin,
    /// The AST before expansion
    pub input: AstNode,
    /// The AST after expansion
    pub output: AstNode,
}

/// Macro expansion environment: holds user/core registries and the trace.
///
/// `MacroExpansionContext` manages the complete macro expansion context:
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
/// use std::sync::Arc;
///
/// use sutra::macros::MacroExpansionContext;
/// let source = Arc::new(miette::NamedSource::new("test", String::new()));
/// let env = MacroExpansionContext::new(source);
/// assert!(env.user_macros.is_empty());
/// assert!(env.core_macros.is_empty());
/// assert!(env.trace.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct MacroExpansionContext {
    /// User-defined macros loaded from files
    pub user_macros: HashMap<String, MacroDefinition>,
    /// Built-in system macros
    pub core_macros: HashMap<String, MacroDefinition>,
    /// Trace of macro expansion steps
    pub trace: Vec<MacroExpansionStep>,
    /// The source code being processed, for error reporting.
    pub source: Arc<NamedSource<String>>,
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
/// Only template macros are serialized. Attempting to serialize a registry with only function macros will result in an empty output.
///
/// # Example
/// ```rust
/// use std::sync::Arc;
///
/// use sutra::{
///     ast::{Expr, Span, Spanned},
///     macros::{MacroFunction, MacroRegistry, MacroTemplate},
/// };
/// let mut reg = MacroRegistry::new();
/// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
/// reg.register("foo", my_macro_fn);
/// let params = sutra::ast::ParamList {
///     required: vec![],
///     rest: None,
///     span: Span::default(),
/// };
/// let body = Box::new(Spanned {
///     value: Arc::new(Expr::Number(0.0, Span::default())),
///     span: Span::default(),
/// });
/// let template = MacroTemplate::new(params, body).unwrap();
/// reg.register_template("bar", template);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: HashMap<String, MacroDefinition>,
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
    /// use std::sync::Arc;
    ///
    /// use sutra::{
    ///     ast::{ParamList, Spanned},
    ///     macros::MacroTemplate,
    /// };
    /// let params = ParamList {
    ///     required: vec!["x".to_string(), "y".to_string()],
    ///     rest: Some("rest".to_string()),
    ///     span: Default::default(),
    /// };
    /// let body = Box::new(Spanned {
    ///     value: Arc::new(sutra::ast::Expr::Number(0.0, sutra::ast::Span::default())),
    ///     span: sutra::ast::Span::default(),
    /// });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// ```
    pub fn new(params: ParamList, body: Box<AstNode>) -> Result<Self, crate::SutraError> {
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

impl MacroExpansionContext {
    /// Creates a new, empty macro environment.
    ///
    /// The environment starts with empty registries and no trace.
    /// Macros can be added to either the user or core registries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use sutra::macros::MacroExpansionContext;
    /// let source = Arc::new(miette::NamedSource::new("test", String::new()));
    /// let env = MacroExpansionContext::new(source);
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
    pub fn with_user_macro(mut self, name: String, def: MacroDefinition) -> Self {
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
    /// use std::sync::Arc;
    ///
    /// use sutra::macros::{MacroExpansionContext, MacroOrigin};
    /// let source = Arc::new(miette::NamedSource::new("test", String::new()));
    /// let env = MacroExpansionContext::new(source);
    /// match env.lookup_macro("my-macro") {
    ///     Some((MacroOrigin::User, _def)) => println!("Found user macro"),
    ///     Some((MacroOrigin::Core, _def)) => println!("Found core macro"),
    ///     None => println!("Macro not found"),
    /// }
    /// ```
    #[inline]
    pub fn lookup_macro(&self, name: &str) -> Option<(MacroOrigin, &MacroDefinition)> {
        self.user_macros
            .get(name)
            .map(|def| (MacroOrigin::User, def))
            .or_else(|| {
                self.core_macros
                    .get(name)
                    .map(|def| (MacroOrigin::Core, def))
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
    /// use std::sync::Arc;
    ///
    /// use sutra::macros::MacroExpansionContext;
    /// let source = Arc::new(miette::NamedSource::new("test", String::new()));
    /// let env = MacroExpansionContext::new(source);
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

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::MacroRegistry;
    /// let registry = MacroRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new function macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `func` - The function that implements the macro
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// let old = reg.register("foo", my_macro_fn).unwrap();
    /// assert!(old.is_none());
    /// let old2 = reg.register("foo", my_macro_fn).unwrap();
    /// assert!(old2.is_some());
    /// ```
    pub fn register(
        &mut self,
        name: &str,
        func: MacroFunction,
    ) -> Result<Option<MacroDefinition>, crate::SutraError> {
        let old_macro = self
            .macros
            .insert(name.to_string(), MacroDefinition::Fn(func));
        Ok(old_macro)
    }

    /// Registers a new function macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register` that prevents accidental overwrites of existing macros.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `func` - The function that implements the macro
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register_or_error("foo", my_macro_fn).unwrap();
    /// assert!(reg.register_or_error("foo", my_macro_fn).is_err());
    /// ```
    pub fn register_or_error(
        &mut self,
        name: &str,
        func: MacroFunction,
    ) -> Result<(), crate::SutraError> {
        if self.macros.contains_key(name) {
            let src_arc = to_error_source(name);
            return Err(err_ctx!(
                Validation,
                format!("Macro '{}' is already registered", name),
                &src_arc,
                Span::default(),
                "Macro already registered"
            ));
        }
        self.macros
            .insert(name.to_string(), MacroDefinition::Fn(func));
        Ok(())
    }

    /// Registers a template macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `template` - The template that defines the macro
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use sutra::{
    ///     ast::{Expr, Span, Spanned},
    ///     macros::{MacroRegistry, MacroTemplate},
    /// };
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList {
    ///     required: vec![],
    ///     rest: None,
    ///     span: Span::default(),
    /// };
    /// let body = Box::new(Spanned {
    ///     value: Arc::new(Expr::Number(0.0, Span::default())),
    ///     span: Span::default(),
    /// });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// let old = reg.register_template("foo", template.clone()).unwrap();
    /// assert!(old.is_none());
    /// let old2 = reg.register_template("foo", template).unwrap();
    /// assert!(old2.is_some());
    /// ```
    pub fn register_template(
        &mut self,
        name: &str,
        template: MacroTemplate,
    ) -> Result<Option<MacroDefinition>, crate::SutraError> {
        let old_macro = self
            .macros
            .insert(name.to_string(), MacroDefinition::Template(template));
        Ok(old_macro)
    }

    /// Registers a template macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register_template` that prevents accidental overwrites of existing macros.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `template` - The template that defines the macro
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use sutra::{
    ///     ast::{Expr, Span, Spanned},
    ///     macros::{MacroRegistry, MacroTemplate},
    /// };
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList {
    ///     required: vec![],
    ///     rest: None,
    ///     span: Span::default(),
    /// };
    /// let body = Box::new(Spanned {
    ///     value: Arc::new(Expr::Number(0.0, Span::default())),
    ///     span: Span::default(),
    /// });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// reg.register_template_or_error("foo", template.clone())
    ///     .unwrap();
    /// assert!(reg.register_template_or_error("foo", template).is_err());
    /// ```
    pub fn register_template_or_error(
        &mut self,
        name: &str,
        template: MacroTemplate,
    ) -> Result<(), crate::SutraError> {
        if self.macros.contains_key(name) {
            let src_arc = to_error_source(name);
            return Err(err_ctx!(
                Validation,
                format!("Macro '{}' is already registered", name),
                &src_arc,
                Span::default(),
                "Macro already registered"
            ));
        }
        self.macros
            .insert(name.to_string(), MacroDefinition::Template(template));
        Ok(())
    }

    /// Unregisters a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro)` if the macro was found and removed, `None` if it didn't exist.
    /// No effect if the macro does not exist.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// let removed = reg.unregister("foo");
    /// assert!(removed.is_some());
    /// assert!(!reg.contains("foo"));
    /// ```
    pub fn unregister(&mut self, name: &str) -> Option<MacroDefinition> {
        self.macros.remove(name)
    }

    /// Looks up a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro_def)` if found, `None` if not found.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// if let Some(_macro_def) = reg.lookup("foo") {
    ///     // Found macro
    /// }
    /// ```
    pub fn lookup(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Checks if a macro with the given name is registered.
    ///
    /// Names are case-sensitive.
    /// Returns `true` if a macro with this name exists, `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// assert!(reg.contains("foo"));
    /// assert!(!reg.contains("nonexistent"));
    /// ```
    pub fn contains(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Returns the number of registered macros.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// assert_eq!(reg.len(), 0);
    /// reg.register("foo", my_macro_fn);
    /// assert_eq!(reg.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.macros.len()
    }

    /// Returns true if the registry is empty.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::MacroRegistry;
    /// let reg = MacroRegistry::new();
    /// assert!(reg.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Returns an iterator over macro names.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("macro1", my_macro_fn);
    /// reg.register("macro2", my_macro_fn);
    /// let names: Vec<_> = reg.names().collect();
    /// assert_eq!(names.len(), 2);
    /// ```
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.macros.keys()
    }

    /// Returns an iterator over macro definitions.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// for (_name, _def) in reg.iter() {
    ///     // ...
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MacroDefinition)> {
        self.macros.iter()
    }

    /// Clears all registered macros.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroFunction, MacroRegistry};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFunction = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// assert!(!reg.is_empty());
    /// reg.clear();
    /// assert!(reg.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.macros.clear();
    }
}

// ============================================================================
// SERIALIZATION SUPPORT
// ============================================================================

impl Serialize for MacroRegistry {
    /// Serializes the registry, including only template macros.
    ///
    /// Function macros are filtered out during serialization since function pointers cannot be serialized.
    /// Attempting to serialize a registry with only function macros will result in an empty output.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use sutra::{
    ///     ast::{Expr, Span, Spanned},
    ///     macros::{MacroRegistry, MacroTemplate},
    /// };
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList {
    ///     required: vec![],
    ///     rest: None,
    ///     span: Span::default(),
    /// };
    /// let body = Box::new(Spanned {
    ///     value: Arc::new(Expr::Number(0.0, Span::default())),
    ///     span: Span::default(),
    /// });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// reg.register_template("foo", template);
    /// let json = serde_json::to_string(&reg).unwrap();
    /// assert!(json.contains("foo"));
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Only serialize Template macros, skip Fn variants
        let template_macros: HashMap<String, &MacroTemplate> = self
            .macros
            .iter()
            .filter_map(|(name, def)| {
                if let MacroDefinition::Template(template) = def {
                    Some((name.clone(), template))
                } else {
                    None
                }
            })
            .collect();

        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("MacroRegistry", 1)?;
        s.serialize_field("macros", &template_macros)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for MacroRegistry {
    /// Deserializes the registry, creating template macros.
    ///
    /// Only template macros are deserialized, as function macros cannot be serialized/deserialized.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use sutra::{
    ///     ast::{Expr, ParamList, Span, Spanned},
    ///     macros::{MacroRegistry, MacroTemplate},
    /// };
    /// // Construct a MacroTemplate and serialize it to JSON
    /// let params = ParamList {
    ///     required: vec![],
    ///     rest: None,
    ///     span: Span::default(),
    /// };
    /// let body = Box::new(Spanned {
    ///     value: Arc::new(Expr::Number(0.0, Span::default())),
    ///     span: Span::default(),
    /// });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// let mut reg = MacroRegistry::new();
    /// reg.register_template("foo", template);
    /// let json = serde_json::to_string(&reg).unwrap();
    /// let reg2: MacroRegistry = serde_json::from_str(&json).unwrap();
    /// assert!(reg2.contains("foo"));
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MacroRegistryHelper {
            macros: HashMap<String, MacroTemplate>,
        }

        let helper = MacroRegistryHelper::deserialize(deserializer)?;
        let macros = helper
            .macros
            .into_iter()
            .map(|(name, template)| (name, MacroDefinition::Template(template)))
            .collect();

        Ok(MacroRegistry { macros })
    }
}

impl Serialize for MacroDefinition {
    /// Serializes macro definitions.
    ///
    /// Only `Template` variants are serializable. Attempting to serialize a `Fn` variant will result in an error at runtime.
    ///
    /// # Example
    /// ```
    /// use sutra::macros::MacroDefinition;
    /// // let fn_macro = MacroDefinition::Fn(my_macro_fn);
    /// // serde_json::to_string(&fn_macro).unwrap(); // This will error
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MacroDefinition::Template(tmpl) => {
                serializer.serialize_newtype_variant("MacroDefinition", 0, "Template", tmpl)
            }
            MacroDefinition::Fn(_) => {
                // Native functions cannot be serialized - this should never be reached
                // when using the MacroRegistry serializer that filters them out
                Err(serde::ser::Error::custom(
                    "Cannot serialize MacroDefinition::Fn variant - use MacroRegistry serialization instead"
                ))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum MacroDefHelper {
    Template(MacroTemplate),
}

impl<'de> Deserialize<'de> for MacroDefinition {
    /// Deserializes macro definitions.
    ///
    /// Only the `Template` variant is deserializable, as function pointers cannot be serialized/deserialized.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match MacroDefHelper::deserialize(deserializer)? {
            MacroDefHelper::Template(tmpl) => Ok(MacroDefinition::Template(tmpl)),
        }
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
fn check_no_duplicate_params(all_names: &[String]) -> Result<(), crate::SutraError> {
    let mut seen = HashSet::new();
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

impl crate::atoms::Callable for MacroDefinition {
    fn call(
        &self,
        _args: &[Value],
        _context: &mut AtomExecutionContext,
        _current_world: &World,
    ) -> Result<(Value, World), crate::SutraError> {
        // Macros operate on AST nodes, not Values, so they cannot be called through the Callable interface
        // This is a design limitation - macros need syntax transformation, not evaluation
        Err(err_msg!(Validation, "Macros cannot be called through Callable interface - they require AST transformation, not evaluation"))
    }
}

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

// Loading operations - re-exported from loader module
use ::std::sync;
// Expansion operations - re-exported from expander module
pub use expander::{
    bind_macro_params, expand_macro_call, expand_macros_recursively, expand_template,
    substitute_template,
};
pub use loader::{
    check_arity, is_macro_definition, load_macros_from_file, parse_macro_definition,
    parse_macros_from_source,
};

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Creates a new macro environment with empty registries.
///
/// This is a convenience function that creates a `MacroExpansionContext` with empty
/// user and core macro registries and no expansion trace.
///
/// # Examples
///
/// ```rust
/// use std::sync::Arc;
///
/// use miette::NamedSource;
/// use sutra::macros::create_macro_env;
///
/// let source = Arc::new(NamedSource::new("test", "".to_string()));
/// let env = create_macro_env(source);
/// assert!(env.user_macros.is_empty());
/// assert!(env.core_macros.is_empty());
/// ```
pub fn create_macro_env(source: sync::Arc<miette::NamedSource<String>>) -> MacroExpansionContext {
    MacroExpansionContext::new(source)
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
