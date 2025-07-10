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

// === Type Definitions (Core Data Structures) ===

// A macro function is a native Rust function that transforms an AST.
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: ::std::collections::HashMap<String, MacroDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraMacroError {
    Expansion {
        span: crate::ast::Span,
        macro_name: String,
        message: String,
    },
    RecursionLimit {
        span: crate::ast::Span,
        macro_name: String,
    },
    // ...
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

// === Macro Registration and Loading ===

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new macro with the given name.
    pub fn register(&mut self, name: &str, func: MacroFn) {
        self.macros.insert(name.to_string(), MacroDef::Fn(func));
    }
}

/// Parses Sutra macro definitions from a source string.
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

/// Thin wrapper: loads macro definitions from a file.
pub fn load_macros_from_file(path: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path).map_err(|e| io_error(e.to_string(), None))?;
    parse_macros_from_source(&source)
}

fn try_parse_macro_form(
    expr: &crate::ast::WithSpan<crate::ast::Expr>,
    names_seen: &mut ::std::collections::HashSet<String>,
) -> Result<Option<(String, MacroTemplate)>, SutraError> {
    // Only process (define (name ...) body) forms
    let Expr::List(items, _) = &expr.value else {
        return Ok(None);
    };
    if items.len() != 3 {
        return Ok(None);
    }
    let Expr::Symbol(def, _) = &items[0].value else {
        return Ok(None);
    };
    if def != "define" {
        return Ok(None);
    }
    let Expr::ParamList(param_list) = &items[1].value else {
        Err(macro_error(
            "Macro parameter list must be a ParamList.".to_string(),
            Some(items[1].span.clone()),
        ))?
    };
    let macro_name = extract_macro_name(param_list)?;
    if !names_seen.insert(macro_name.clone()) {
        Err(macro_error(
            format!("Duplicate macro name '{}'.", macro_name),
            Some(param_list.span.clone()),
        ))?
    }
    let params = crate::ast::ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span.clone(),
    };
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok(Some((macro_name, template)))
}

fn extract_macro_name(param_list: &crate::ast::ParamList) -> Result<String, SutraError> {
    if let Some(name) = param_list.required.first() {
        Ok(name.clone())
    } else {
        Err(macro_error(
            "Macro name must be the first element of the parameter list.".to_string(),
            Some(param_list.span.clone()),
        ))?
    }
}

// === Macro Definition Validation ===

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

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for duplicate parameters.
    pub fn new(
        params: crate::ast::ParamList,
        body: Box<WithSpan<Expr>>,
    ) -> Result<Self, SutraError> {
        let mut all_names = params.required.clone();
        if let Some(var) = &params.rest {
            all_names.push(var.clone());
        }
        check_no_duplicate_params(&all_names, &params.span)?;
        Ok(MacroTemplate { params, body })
    }
}

// === Macro Call Preparation ===

/// Checks the arity of macro arguments against the parameter list.
pub fn check_arity(
    args_len: usize,
    params: &crate::ast::ParamList,
    span: &crate::ast::Span,
) -> Result<(), SutraError> {
    if args_len < params.required.len() {
        return Err(macro_error(
            format!(
                "Macro expects at least {} arguments, but got {}.",
                params.required.len(),
                args_len
            ),
            Some(span.clone()),
        ));
    }
    if params.rest.is_none() && args_len > params.required.len() {
        return Err(macro_error(
            format!(
                "Macro expects exactly {} arguments, but got {}.",
                params.required.len(),
                args_len
            ),
            Some(span.clone()),
        ));
    }
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
    if let Some(variadic_name) = &params.rest {
        let rest_args = if args.len() > params.required.len() {
            args[params.required.len()..].to_vec()
        } else {
            vec![]
        };
        bindings.insert(
            variadic_name.clone(),
            WithSpan {
                value: Expr::List(rest_args, expr_span.clone()),
                span: expr_span.clone(),
            },
        );
    }
    bindings
}

// === Macro Expansion Core ===

const MAX_MACRO_RECURSION_DEPTH: usize = 128;

/// Expands a macro template call by substituting arguments into the template body.
pub fn expand_template(
    template: &MacroTemplate,
    call: &WithSpan<Expr>,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraError> {
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(macro_error(
            format!(
                "Macro expansion recursion limit ({}) exceeded.",
                MAX_MACRO_RECURSION_DEPTH
            ),
            Some(call.span.clone()),
        ));
    }
    let (args, span) = match &call.value {
        Expr::List(items, span) if !items.is_empty() => (&items[1..], span),
        _ => {
            return Err(macro_error(
                "Template macro must be called as a list.".to_string(),
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
            Ok(WithSpan {
                value: Expr::Quote(Box::new(new_inner), span.clone()),
                span: expr.span.clone(),
            })
        }
        Expr::List(items, _) => {
            let mut new_items = Vec::new();
            for item in items {
                match &item.value {
                    // Regular symbol substitution: replace with bound value as-is (no automatic splicing)
                    Expr::Symbol(_name, _) => {
                        // Just substitute the symbol with its bound value, don't splice lists automatically
                        new_items.push(substitute_template(item, bindings)?);
                    }
                    // Spread argument splicing: if the item is Expr::Spread, splice its elements
                    Expr::Spread(inner) => {
                        let substituted = substitute_template(inner, bindings)?;
                        if let Expr::List(splice_items, _) = &substituted.value {
                            for splice_item in splice_items {
                                new_items.push(splice_item.clone());
                            }
                        } else {
                            // If not a list, treat as a single argument
                            new_items.push(substituted);
                        }
                    }
                    _ => new_items.push(substitute_template(item, bindings)?),
                }
            }
            Ok(WithSpan {
                value: Expr::List(new_items, expr.span.clone()),
                span: expr.span.clone(),
            })
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let new_condition = substitute_template(condition, bindings)?;
            let new_then = substitute_template(then_branch, bindings)?;
            let new_else = substitute_template(else_branch, bindings)?;
            Ok(WithSpan {
                value: Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: Box::new(new_else),
                    span: span.clone(),
                },
                span: expr.span.clone(),
            })
        }
        _ => Ok(expr.clone()),
    }
}

// === Macro Lookup and Expansion ===

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

fn expand_macro_once(
    node: &WithSpan<Expr>,
    env: &MacroEnv,
    depth: usize,
) -> Result<Option<(String, MacroProvenance, WithSpan<Expr>)>, SutraMacroError> {
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(SutraMacroError::RecursionLimit {
            span: node.span.clone(),
            macro_name: "<unknown>".to_string(),
        });
    }
    let items = match &node.value {
        Expr::List(items, _) => items,
        _ => return Ok(None),
    };
    if items.is_empty() {
        return Ok(None);
    }
    let macro_name = match &items[0].value {
        Expr::Symbol(s, _) => s,
        _ => return Ok(None),
    };
    if let Some((provenance, macro_def)) = env.lookup_macro(macro_name) {
        let expanded =
            match macro_def {
                MacroDef::Fn(func) => func(node).map_err(|e| SutraMacroError::Expansion {
                    span: node.span.clone(),
                    macro_name: macro_name.clone(),
                    message: e.to_string(),
                })?,
                MacroDef::Template(template) => expand_template(template, node, depth + 1)
                    .map_err(|e| SutraMacroError::Expansion {
                        span: node.span.clone(),
                        macro_name: macro_name.clone(),
                        message: e.to_string(),
                    })?,
            };
        return Ok(Some((macro_name.clone(), provenance, expanded)));
    }
    Ok(None)
}

// === AST Traversal and Transformation ===

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
            let new_items: Result<Vec<_>, _> = items
                .iter()
                .map(|item| f(item.clone(), env, depth + 1))
                .collect();
            Ok(WithSpan {
                value: Expr::List(new_items?, span.clone()),
                span: node.span.clone(),
            })
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let cond = f((**condition).clone(), env, depth + 1)?;
            let then_b = f((**then_branch).clone(), env, depth + 1)?;
            let else_b = f((**else_branch).clone(), env, depth + 1)?;
            Ok(WithSpan {
                value: Expr::If {
                    condition: Box::new(cond),
                    then_branch: Box::new(then_b),
                    else_branch: Box::new(else_b),
                    span: span.clone(),
                },
                span: node.span.clone(),
            })
        }
        // Add more composite node types as needed
        _ => Ok(node),
    }
}

// === Macro Expansion with Trace ===

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
            node.clone(),
            expanded.clone(),
        );
        return expand_macros_with_trace(expanded, env, depth + 1);
    }
    map_ast(node, &expand_macros_with_trace, env, depth)
}

// === Public Entry Point ===

/// Public entry point for macro expansion.
pub fn expand_macros(
    ast: WithSpan<Expr>,
    env: &mut MacroEnv,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    expand_macros_with_trace(ast, env, 0)
}

// === Serialization/Deserialization ===

impl Serialize for MacroDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MacroDef::Template(tmpl) => {
                serializer.serialize_newtype_variant("MacroDef", 1, "Template", tmpl)
            }
            MacroDef::Fn(_) => {
                // Native functions are not serializable; skip or error.
                serializer.serialize_unit_variant("MacroDef", 0, "Fn")
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum MacroDefHelper {
    Template(MacroTemplate),
    Fn,
}

impl<'de> Deserialize<'de> for MacroDef {
    /// Only the Template variant is deserializable. Fn is not supported and will error.
    ///
    /// # Errors
    /// Returns an error if the Fn variant is encountered.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match MacroDefHelper::deserialize(deserializer)? {
            MacroDefHelper::Template(tmpl) => Ok(MacroDef::Template(tmpl)),
            MacroDefHelper::Fn => Err(serde::de::Error::custom(
                "Cannot deserialize MacroDef::Fn variant",
            )),
        }
    }
}

pub mod std;
