//! # Sutra Macro Expansion System
//!
//! This module is responsible for the purely syntactic transformation of the AST
//! before evaluation. Macros allow authors to create high-level abstractions
//! that expand into simpler, core expressions.
//!
//! ## Core Principles
//!
//! - **Syntactic Only**: Macros operate solely on the AST (`WithSpan<Expr>`). They have no access
//!   to the `World` state and cannot perform any evaluation or side effects.
//! - **Pure Transformation**: Macro expansion is a pure function: `(WithSpan<Expr>) -> Result<WithSpan<Expr>, Error>`.
//! - **Inspectable**: The expansion process can be traced, allowing authors to see
//!   how their high-level forms are desugared into core language constructs.
//! - **Layered**: The macro system is a distinct pipeline stage that runs after parsing
//!   and before validation and evaluation.
//!
//! **INVARIANT:** All macro system logic, macro functions, and recursive expansion must operate on `WithSpan<Expr>`. Never unwrap to a bare `Expr` except for internal logic, and always re-wrap with the correct span. All lists are `Vec<WithSpan<Expr>>`.
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

use crate::ast::{Expr, WithSpan};
use crate::syntax::error::{io_error, macro_error, SutraError};
use ::std::fs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ============================================================================
// SECTION 1: CORE DATA STRUCTURES
// ============================================================================

/// A macro function is a native Rust function that transforms an AST.
pub type MacroFn =
    fn(
        &crate::ast::WithSpan<crate::ast::Expr>,
    ) -> Result<crate::ast::WithSpan<crate::ast::Expr>, crate::syntax::error::SutraError>;

/// A declarative macro defined by a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroTemplate {
    pub params: crate::ast::ParamList,
    pub body: Box<WithSpan<Expr>>,
}

/// MacroDef cannot serialize/deserialize native function pointers. Only Template variant is serializable.
#[derive(Debug, Clone)]
pub enum MacroDef {
    Fn(MacroFn),
    Template(MacroTemplate),
}

/// Macro registry for built-in and template macros.
#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: ::std::collections::HashMap<String, MacroDef>,
}

/// Macro expansion errors with contextual information.
///
/// The `Expansion` variant now preserves structured error information from the underlying
/// error instead of flattening it to a string, allowing better debugging and error handling.
///
/// # Examples
///
/// ```rust
/// use sutra::macros::SutraMacroError;
///
/// // Access structured error information for debugging
/// if let SutraMacroError::Expansion { suggestion, source_error_kind, .. } = error {
///     if let Some(suggestion) = suggestion {
///         println!("Suggestion: {}", suggestion);
///     }
///     // Access the original error type for specific handling
///     match source_error_kind {
///         Some(crate::syntax::error::SutraErrorKind::Eval(eval_err)) => {
///             println!("Eval error with code: {}", eval_err.expanded_code);
///         }
///         _ => {}
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraMacroError {
    Expansion {
        span: crate::ast::Span,
        macro_name: String,
        message: String,
        /// Preserved source error for structured information access.
        /// Contains the original error kind that caused the macro expansion failure,
        /// allowing access to detailed error information like suggestions and expanded code.
        source_error_kind: Option<crate::syntax::error::SutraErrorKind>,
        /// Optional suggestion from the original error for better debugging.
        /// Extracted from EvalError when available to provide actionable guidance.
        suggestion: Option<String>,
        /// Original span from the source error, if different from macro call span.
        /// Helps pinpoint the exact location where the underlying error occurred.
        source_span: Option<crate::ast::Span>,
    },
    RecursionLimit {
        span: crate::ast::Span,
        macro_name: String,
    },
}

/// Provenance of a macro expansion step: user or core registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroProvenance {
    User,
    Core,
}

/// A single macro expansion step, for traceability.
#[derive(Debug, Clone)]
pub struct MacroExpansionStep {
    /// The macro name invoked.
    pub macro_name: String,
    /// Which registry the macro was found in.
    pub provenance: MacroProvenance,
    /// The AST before expansion.
    pub input: WithSpan<Expr>,
    /// The AST after expansion.
    pub output: WithSpan<Expr>,
}

/// Macro expansion environment: holds user/core registries and the trace.
#[derive(Debug, Clone)]
pub struct MacroEnv {
    pub user_macros: ::std::collections::HashMap<String, MacroDef>,
    pub core_macros: ::std::collections::HashMap<String, MacroDef>,
    pub trace: Vec<MacroExpansionStep>,
}

const MAX_MACRO_RECURSION_DEPTH: usize = 128;

// ============================================================================
// SECTION 2: PUBLIC API IMPLEMENTATION
// ============================================================================

// --- Macro Registry Operations ---

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new macro with the given name.
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    /// This allows callers to detect silent overwrites.
    pub fn register(&mut self, name: &str, func: MacroFn) -> Option<MacroDef> {
        self.macros.insert(name.to_string(), MacroDef::Fn(func))
    }

    /// Registers a new macro with the given name, returning an error if it already exists.
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered.
    pub fn register_or_error(&mut self, name: &str, func: MacroFn) -> Result<(), String> {
        if self.macros.contains_key(name) {
            return Err(format!("Macro '{}' is already registered", name));
        }
        self.macros.insert(name.to_string(), MacroDef::Fn(func));
        Ok(())
    }

    /// Registers a template macro with the given name.
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    pub fn register_template(&mut self, name: &str, template: MacroTemplate) -> Option<MacroDef> {
        self.macros.insert(name.to_string(), MacroDef::Template(template))
    }

    /// Unregisters a macro by name.
    ///
    /// # Returns
    /// `Some(macro)` if the macro was found and removed, `None` if it didn't exist.
    pub fn unregister(&mut self, name: &str) -> Option<MacroDef> {
        self.macros.remove(name)
    }

    /// Checks if a macro with the given name is registered.
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
}

// --- Macro Loading and Parsing ---

/// Parses Sutra macro definitions from a source string.
///
/// # Examples
/// ```rust
/// # use sutra::macros::parse_macros_from_source;
/// let source = "(define (my-macro x) (+ x 1))";
/// let macros = parse_macros_from_source(source)?;
/// assert_eq!(macros.len(), 1);
/// assert_eq!(macros[0].0, "my-macro");
/// # Ok::<(), sutra::syntax::error::SutraError>(())
/// ```
pub fn parse_macros_from_source(source: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let exprs = crate::syntax::parser::parse(source)?;
    let mut macros = Vec::new();
    let mut names_seen = ::std::collections::HashSet::new();

    for expr in exprs {
        if let Some((macro_name, template)) = try_parse_macro_form(&expr, &mut names_seen)? {
            macros.push((macro_name, template));
        }
    }
    Ok(macros)
}

/// Loads macro definitions from a file with improved path ergonomics.
///
/// # Examples
/// ```rust
/// # use sutra::macros::load_macros_from_file;
/// # use std::path::Path;
/// // Works with &str
/// let macros1 = load_macros_from_file("macros.sutra");
/// // Works with Path
/// let macros2 = load_macros_from_file(std::path::Path::new("macros.sutra"));
/// // Works with PathBuf, &Path, etc.
/// let macros3 = load_macros_from_file(&std::path::PathBuf::from("macros.sutra"));
/// ```
pub fn load_macros_from_file<P: AsRef<::std::path::Path>>(path: P) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path).map_err(|e| io_error(e.to_string(), None))?;
    parse_macros_from_source(&source)
}

// --- Macro Expansion Core ---

/// Checks the arity of macro arguments against the parameter list with enhanced error reporting.
pub fn check_arity(
    args_len: usize,
    params: &crate::ast::ParamList,
    span: &crate::ast::Span,
) -> Result<(), SutraError> {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();

    // Too few arguments
    if args_len < required_len {
        return Err(enhanced_macro_arity_error(args_len, params, span));
    }

    // Too many arguments for non-variadic macro
    if args_len > required_len && !has_variadic {
        return Err(enhanced_macro_arity_error(args_len, params, span));
    }

    // Arity is correct
    Ok(())
}

/// Binds macro parameters to arguments, returning a map from parameter name to argument value.
pub fn bind_macro_params(
    args: &[WithSpan<Expr>],
    params: &crate::ast::ParamList,
    expr_span: &crate::ast::Span,
) -> ::std::collections::HashMap<String, WithSpan<Expr>> {
    let mut bindings = ::std::collections::HashMap::new();
    for (i, param_name) in params.required.iter().enumerate() {
        bindings.insert(param_name.clone(), args[i].clone());
    }

    // Handle variadic parameters if present
    let Some(variadic_name) = &params.rest else {
        return bindings;
    };

    let rest_args = if args.len() > params.required.len() {
        args[params.required.len()..].to_vec()
    } else {
        Vec::new()
    };
    bindings.insert(
        variadic_name.clone(),
        with_span(Expr::List(rest_args, expr_span.clone()), expr_span),
    );
    bindings
}

/// Expands a macro template call by substituting arguments into the template body.
pub fn expand_template(
    template: &MacroTemplate,
    call: &WithSpan<Expr>,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraError> {
    check_recursion_depth(depth, &call.span, "Macro expansion")?;
    let (args, span) = match &call.value {
        Expr::List(items, span) if !items.is_empty() => (&items[1..], span),
        _ => {
            return Err(macro_error(
                "Template macro must be called as a list.",
                Some(call.span.clone()),
            ));
        }
    };
    check_arity(args.len(), &template.params, span)?;
    let bindings = bind_macro_params(args, &template.params, span);
    substitute_template(&template.body, &bindings)
}

/// Recursively substitutes macro parameters in the template body with provided arguments.
pub fn substitute_template(
    expr: &WithSpan<Expr>,
    bindings: &::std::collections::HashMap<String, WithSpan<Expr>>,
) -> Result<WithSpan<Expr>, SutraError> {
    match &expr.value {
        Expr::Symbol(name, _span) => {
            Ok(bindings.get(name).cloned().unwrap_or_else(|| expr.clone()))
        }
        Expr::Quote(inner, span) => {
            let new_inner = substitute_template(inner, bindings)?;
            Ok(with_span(Expr::Quote(Box::new(new_inner), span.clone()), &expr.span))
        }
        Expr::List(items, _) => {
            substitute_list(items, bindings, &expr.span)
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            substitute_if(condition, then_branch, else_branch, bindings, span, &expr.span)
        }
        _ => Ok(expr.clone()),
    }
}

/// Public entry point for macro expansion.
pub fn expand_macros(
    ast: WithSpan<Expr>,
    env: &mut MacroEnv,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    expand_macros_with_trace(ast, env, 0)
}

// --- Macro Environment Operations ---

impl MacroEnv {
    /// Looks up a macro by name, returning provenance and definition.
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
    pub fn trace(&self) -> &[MacroExpansionStep] {
        &self.trace
    }
}

// ============================================================================
// SECTION 3: CONVERSIONS
// ============================================================================

// (No conversions needed for this module)

// ============================================================================
// SECTION 4: INFRASTRUCTURE/TRAITS
// ============================================================================

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for duplicate parameters.
    pub fn new(
        params: crate::ast::ParamList,
        body: Box<WithSpan<Expr>>,
    ) -> Result<Self, SutraError> {
        let mut all_names = params.required.clone();

        // Add variadic parameter if present
        if let Some(var) = &params.rest {
            all_names.push(var.clone());
        }

        // Validate no duplicate parameters
        check_no_duplicate_params(&all_names, &params.span)?;

        // Construct template
        Ok(MacroTemplate { params, body })
    }
}

impl Serialize for MacroDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MacroDef::Template(tmpl) => {
                serializer.serialize_newtype_variant("MacroDef", 0, "Template", tmpl)
            }
            MacroDef::Fn(_) => {
                // Native functions cannot be serialized - this should never be reached
                // when using the MacroRegistry serializer that filters them out
                Err(serde::ser::Error::custom(
                    "Cannot serialize MacroDef::Fn variant - use MacroRegistry serialization instead"
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

impl<'de> Deserialize<'de> for MacroDef {
    /// Only the Template variant is deserializable.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match MacroDefHelper::deserialize(deserializer)? {
            MacroDefHelper::Template(tmpl) => Ok(MacroDef::Template(tmpl)),
        }
    }
}

impl Serialize for MacroRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Only serialize Template macros, skip Fn variants
        let template_macros: ::std::collections::HashMap<String, &MacroTemplate> = self
            .macros
            .iter()
            .filter_map(|(name, def)| {
                if let MacroDef::Template(template) = def {
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MacroRegistryHelper {
            macros: ::std::collections::HashMap<String, MacroTemplate>,
        }

        let helper = MacroRegistryHelper::deserialize(deserializer)?;
        let macros = helper
            .macros
            .into_iter()
            .map(|(name, template)| (name, MacroDef::Template(template)))
            .collect();

        Ok(MacroRegistry { macros })
    }
}

// ============================================================================
// SECTION 5: INTERNAL HELPERS
// ============================================================================

// --- Macro Definition Parsing ---

/// Validates basic structure of a (define (name ...) body) form
fn validate_define_form(expr: &WithSpan<Expr>) -> Option<&[WithSpan<Expr>]> {
    let Some(items) = extract_list_items(expr) else {
        return None;
    };
    if items.len() != 3 {
        return None;
    }
    let Some(def) = extract_symbol_from_expr(&items[0]) else {
        return None;
    };
    if def != "define" {
        return None;
    }
    Some(items)
}

/// Extracts and validates macro name, checking for duplicates
fn extract_and_check_macro_name(
    param_list: &crate::ast::ParamList,
    names_seen: &mut ::std::collections::HashSet<String>,
) -> Result<String, SutraError> {
    let macro_name = extract_macro_name(param_list)?;
    if !names_seen.insert(macro_name.clone()) {
        return Err(macro_error(
            format!("Duplicate macro name '{}'.", macro_name),
            Some(param_list.span.clone()),
        ));
    }
    Ok(macro_name)
}

/// Builds macro parameters by removing the name from the parameter list
fn build_macro_params(param_list: &crate::ast::ParamList) -> crate::ast::ParamList {
    crate::ast::ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span.clone(),
    }
}

fn try_parse_macro_form(
    expr: &crate::ast::WithSpan<crate::ast::Expr>,
    names_seen: &mut ::std::collections::HashSet<String>,
) -> Result<Option<(String, MacroTemplate)>, SutraError> {
    // Validate basic define form structure
    let Some(items) = validate_define_form(expr) else {
        return Ok(None);
    };

    // Ensure parameter list is valid
    let Expr::ParamList(param_list) = &items[1].value else {
        return Err(macro_error(
            "Macro parameter list must be a ParamList.",
            Some(items[1].span.clone()),
        ));
    };

    let macro_name = extract_and_check_macro_name(param_list, names_seen)?;
    let params = build_macro_params(param_list);
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok(Some((macro_name, template)))
}

fn extract_macro_name(param_list: &crate::ast::ParamList) -> Result<String, SutraError> {
    // Ensure parameter list has at least one element
    let Some(name) = param_list.required.first() else {
        return Err(macro_error(
            "Macro name must be the first element of the parameter list.",
            Some(param_list.span.clone()),
        ));
    };

    Ok(name.clone())
}

fn check_no_duplicate_params(
    all_names: &[String],
    span: &crate::ast::Span,
) -> Result<(), SutraError> {
    let mut seen = ::std::collections::HashSet::new();
    for name in all_names {
        if !seen.insert(name) {
            return Err(macro_error(
                format!("Duplicate parameter name '{}' in macro definition.", name),
                Some(span.clone()),
            ));
        }
    }
    Ok(())
}

// --- Arity and Binding Helpers ---

/// Builds arity error context message - DRY utility
fn build_arity_context_message(args_len: usize, required_len: usize, has_variadic: bool) -> String {
    // Handle variadic case early
    if has_variadic {
        return format!(
            "Expected at least {} arguments, but received {}. This macro accepts additional arguments via '...' parameter.",
            required_len, args_len
        );
    }

    // Exact argument count required
    format!(
        "Expected exactly {} arguments, but received {}. This macro requires a specific number of arguments.",
        required_len, args_len
    )
}

/// Builds parameter info string - DRY utility
fn build_param_info_string(params: &crate::ast::ParamList) -> String {
    format!(
        "Macro parameters: {}{}",
        params.required.join(", "),
        if let Some(rest) = &params.rest {
            format!(" ...{}", rest)
        } else {
            String::new()
        }
    )
}

/// Helper for pluralizing argument count messages - DRY utility.
fn pluralize_args(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Generates arity error suggestions - DRY utility
fn build_arity_suggestion(
    args_len: usize,
    required_len: usize,
    has_variadic: bool,
    param_info: &str,
) -> String {
    // Too few arguments
    if args_len < required_len {
        let missing = required_len - args_len;
        return format!(
            "Add {} more argument{} to match the macro definition: {}",
            missing,
            pluralize_args(missing),
            param_info
        );
    }

    // Too many arguments (non-variadic)
    if args_len > required_len && !has_variadic {
        let extra = args_len - required_len;
        return format!(
            "Remove {} argument{} - this macro only accepts {} arguments: {}",
            extra,
            pluralize_args(extra),
            required_len,
            param_info
        );
    }

    // General mismatch
    format!("Check the macro definition and ensure arguments match: {}", param_info)
}

/// Creates enhanced arity error - DRY utility
fn enhanced_macro_arity_error(
    args_len: usize,
    params: &crate::ast::ParamList,
    span: &crate::ast::Span,
) -> SutraError {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();

    let main_message = "Macro arity mismatch";
    let context_message = build_arity_context_message(args_len, required_len, has_variadic);
    let param_info = build_param_info_string(params);
    let suggestion = build_arity_suggestion(args_len, required_len, has_variadic, &param_info);

    let full_message = format!("{}\n\n{}\n\n{}", main_message, context_message, param_info);

    use crate::syntax::error::{SutraError, SutraErrorKind, EvalError};
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: full_message,
            expanded_code: format!("<macro call with {} arguments>", args_len),
            original_code: None,
            suggestion: Some(suggestion),
        }),
        span: Some(span.clone()),
    }
}

// --- Validation Helpers ---

/// Generic recursion depth checker - DRY utility
fn check_recursion_depth_generic<E>(
    depth: usize,
    span: &crate::ast::Span,
    error_fn: impl FnOnce(&crate::ast::Span) -> E,
) -> Result<(), E> {
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(error_fn(span));
    }
    Ok(())
}

/// Checks recursion depth limit - DRY utility
fn check_recursion_depth(
    depth: usize,
    span: &crate::ast::Span,
    context: &str,
) -> Result<(), SutraError> {
    check_recursion_depth_generic(depth, span, |span| {
        macro_error(
            format!(
                "{} recursion limit ({}) exceeded.",
                context,
                MAX_MACRO_RECURSION_DEPTH
            ),
            Some(span.clone()),
        )
    })
}

/// Checks recursion depth limit for macro operations - DRY utility
fn check_macro_recursion_depth(
    depth: usize,
    span: &crate::ast::Span,
    macro_name: Option<&str>,
) -> Result<(), SutraMacroError> {
    check_recursion_depth_generic(depth, span, |span| {
        SutraMacroError::RecursionLimit {
            span: span.clone(),
            macro_name: macro_name.unwrap_or("<unknown>").to_string(),
        }
    })
}

/// Extracts list items from expression - DRY utility
fn extract_list_items(expr: &WithSpan<Expr>) -> Option<&[WithSpan<Expr>]> {
    let Expr::List(items, _) = &expr.value else {
        return None;
    };
    Some(items)
}

/// Extracts symbol from an expression - DRY utility
fn extract_symbol_from_expr(expr: &WithSpan<Expr>) -> Option<&str> {
    let Expr::Symbol(s, _) = &expr.value else {
        return None;
    };
    Some(s)
}

// --- Error Handling Helpers ---

/// Creates macro expansion error preserving context
fn expansion_error_from_sutra_error(
    span: &crate::ast::Span,
    macro_name: &str,
    error: SutraError,
) -> SutraMacroError {
    // Extract structured information from the source error before losing it
    let suggestion = match &error.kind {
        crate::syntax::error::SutraErrorKind::Eval(eval_error) => eval_error.suggestion.clone(),
        _ => None,
    };

    // Create enhanced message that still provides human-readable context
    let enhanced_message = format!(
        "Macro expansion failed: {}{}",
        error,
        error.span.as_ref().map_or(String::new(), |s| format!(" (at {}:{})", s.start, s.end))
    );

    SutraMacroError::Expansion {
        span: span.clone(),
        macro_name: macro_name.to_string(),
        message: enhanced_message,
        source_error_kind: Some(error.kind),
        suggestion,
        source_span: error.span,
    }
}

// --- Macro Expansion Logic ---

/// Extracts macro name from call node - DRY utility
fn extract_macro_name_from_call(node: &WithSpan<Expr>) -> Option<&str> {
    let Some(items) = extract_list_items(node) else {
        return None;
    };
    if items.is_empty() {
        return None;
    }
    extract_symbol_from_expr(&items[0])
}

/// Expands macro definition - DRY utility
fn expand_macro_def(
    macro_def: &MacroDef,
    node: &WithSpan<Expr>,
    macro_name: &str,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    let result = match macro_def {
        MacroDef::Fn(func) => func(node),
        MacroDef::Template(template) => expand_template(template, node, depth + 1),
    };

    // Preserve structured error context instead of losing it via .to_string()
    result.map_err(|e| expansion_error_from_sutra_error(&node.span, macro_name, e))
}

/// Expands macro once with depth checking
fn expand_macro_once(
    node: &WithSpan<Expr>,
    env: &MacroEnv,
    depth: usize,
) -> Result<Option<(String, MacroProvenance, WithSpan<Expr>)>, SutraMacroError> {
    // Extract macro name from call first to provide better error context
    let Some(macro_name) = extract_macro_name_from_call(node) else {
        return Ok(None);
    };

    // Check recursion depth with actual macro name
    check_macro_recursion_depth(depth, &node.span, Some(macro_name))?;

    // Lookup macro definition
    let Some((provenance, macro_def)) = env.lookup_macro(macro_name) else {
        return Ok(None);
    };

    let expanded = expand_macro_def(macro_def, node, macro_name, depth)?;
    Ok(Some((macro_name.to_string(), provenance, expanded)))
}

// --- AST Traversal Helpers ---

/// Maps function over List items - AST helper
fn map_list<F>(
    items: &[WithSpan<Expr>],
    f: &F,
    env: &mut MacroEnv,
    depth: usize,
    original_span: &crate::ast::Span,
    list_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraMacroError>
where
    F: Fn(WithSpan<Expr>, &mut MacroEnv, usize) -> Result<WithSpan<Expr>, SutraMacroError>,
{
    let new_items: Result<Vec<_>, _> = items
        .iter()
        .map(|item| f(item.clone(), env, depth + 1))
        .collect();
    Ok(with_span(
        Expr::List(new_items?, list_span.clone()),
        original_span,
    ))
}

/// Maps function over If branches - AST helper
fn map_if<F>(
    condition: &WithSpan<Expr>,
    then_branch: &WithSpan<Expr>,
    else_branch: &WithSpan<Expr>,
    f: &F,
    env: &mut MacroEnv,
    depth: usize,
    if_span: &crate::ast::Span,
    original_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraMacroError>
where
    F: Fn(WithSpan<Expr>, &mut MacroEnv, usize) -> Result<WithSpan<Expr>, SutraMacroError>,
{
    let cond = f(condition.clone(), env, depth + 1)?;
    let then_b = f(then_branch.clone(), env, depth + 1)?;
    let else_b = f(else_branch.clone(), env, depth + 1)?;
    Ok(with_span(
        Expr::If {
            condition: Box::new(cond),
            then_branch: Box::new(then_b),
            else_branch: Box::new(else_b),
            span: if_span.clone(),
        },
        original_span,
    ))
}

/// Maps function over Quote content - AST helper
fn map_quote<F>(
    inner: &WithSpan<Expr>,
    f: &F,
    env: &mut MacroEnv,
    depth: usize,
    quote_span: &crate::ast::Span,
    original_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraMacroError>
where
    F: Fn(WithSpan<Expr>, &mut MacroEnv, usize) -> Result<WithSpan<Expr>, SutraMacroError>,
{
    let new_inner = f(inner.clone(), env, depth + 1)?;
    Ok(with_span(
        Expr::Quote(Box::new(new_inner), quote_span.clone()),
        original_span,
    ))
}

/// Maps function over Spread content - AST helper
fn map_spread<F>(
    inner: &WithSpan<Expr>,
    f: &F,
    env: &mut MacroEnv,
    depth: usize,
    original_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraMacroError>
where
    F: Fn(WithSpan<Expr>, &mut MacroEnv, usize) -> Result<WithSpan<Expr>, SutraMacroError>,
{
    let new_inner = f(inner.clone(), env, depth + 1)?;
    Ok(with_span(
        Expr::Spread(Box::new(new_inner)),
        original_span,
    ))
}

/// Maps function over AST nodes recursively
fn map_ast<F>(
    node: WithSpan<Expr>,
    f: &F,
    env: &mut MacroEnv,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError>
where
    F: Fn(WithSpan<Expr>, &mut MacroEnv, usize) -> Result<WithSpan<Expr>, SutraMacroError>,
{
    match &node.value {
        Expr::List(items, span) => {
            map_list(items, f, env, depth, &node.span, span)
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            map_if(condition, then_branch, else_branch, f, env, depth, span, &node.span)
        }
        Expr::Quote(inner, span) => {
            map_quote(inner, f, env, depth, span, &node.span)
        }
        Expr::Spread(inner) => {
            map_spread(inner, f, env, depth, &node.span)
        }
        // ParamList doesn't contain nested expressions
        // Atomic types (Symbol, Path, String, Number, Bool) don't need traversal
        _ => Ok(node),
    }
}

// --- Template Substitution Helpers ---

/// Creates a WithSpan wrapper with consistent span handling - DRY utility
fn with_span(value: Expr, original_span: &crate::ast::Span) -> WithSpan<Expr> {
    WithSpan {
        value,
        span: original_span.clone(),
    }
}

/// Substitutes parameters in List with splicing
fn substitute_list(
    items: &[WithSpan<Expr>],
    bindings: &::std::collections::HashMap<String, WithSpan<Expr>>,
    original_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraError> {
    let mut new_items = Vec::new();
    for item in items {
        match &item.value {
            // Spread argument splicing requires special handling for list elements
            Expr::Spread(inner) => {
                substitute_spread_item(inner, bindings, &mut new_items)?;
            }
            // All other expressions get regular substitution (Symbol, literals, etc.)
            _ => {
                new_items.push(substitute_template(item, bindings)?);
            }
        }
    }
    Ok(with_span(
        Expr::List(new_items, original_span.clone()),
        original_span,
    ))
}

/// Handles spread argument substitution with guard clauses - focused helper
fn substitute_spread_item(
    inner: &WithSpan<Expr>,
    bindings: &::std::collections::HashMap<String, WithSpan<Expr>>,
    new_items: &mut Vec<WithSpan<Expr>>,
) -> Result<(), SutraError> {
    let substituted = substitute_template(inner, bindings)?;

    // If not a list, treat as single argument
    let Expr::List(splice_items, _) = &substituted.value else {
        new_items.push(substituted);
        return Ok(());
    };

    // Splice list elements into parent
    new_items.extend(splice_items.iter().cloned());
    Ok(())
}

/// Substitutes parameters in If expression
fn substitute_if(
    condition: &WithSpan<Expr>,
    then_branch: &WithSpan<Expr>,
    else_branch: &WithSpan<Expr>,
    bindings: &::std::collections::HashMap<String, WithSpan<Expr>>,
    if_span: &crate::ast::Span,
    original_span: &crate::ast::Span,
) -> Result<WithSpan<Expr>, SutraError> {
    let new_condition = substitute_template(condition, bindings)?;
    let new_then = substitute_template(then_branch, bindings)?;
    let new_else = substitute_template(else_branch, bindings)?;
    Ok(with_span(
        Expr::If {
            condition: Box::new(new_condition),
            then_branch: Box::new(new_then),
            else_branch: Box::new(new_else),
            span: if_span.clone(),
        },
        original_span,
    ))
}

// --- Tracing and Debugging Helpers ---

/// Records macro expansion step in trace
fn record_macro_expansion(
    trace: &mut Vec<MacroExpansionStep>,
    macro_name: String,
    provenance: MacroProvenance,
    input: WithSpan<Expr>,
    output: WithSpan<Expr>,
) {
    trace.push(MacroExpansionStep {
        macro_name,
        provenance,
        input,
        output,
    });
}

/// Expands macros with trace recording
fn expand_macros_with_trace(
    node: WithSpan<Expr>,
    env: &mut MacroEnv,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    if let Some((macro_name, provenance, expanded)) = expand_macro_once(&node, env, depth)? {
        record_macro_expansion(
            &mut env.trace,
            macro_name,
            provenance,
            node,
            expanded.clone(),
        );
        return expand_macros_with_trace(expanded, env, depth + 1);
    }
    map_ast(node, &expand_macros_with_trace, env, depth)
}

// ============================================================================
// SECTION 6: MODULE EXPORTS
// ============================================================================

pub mod std;
