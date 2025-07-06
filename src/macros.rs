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
//! **INVARIANT:** All macroexpander logic, macro functions, and recursive expansion must operate on `WithSpan<Expr>`. Never unwrap to a bare `Expr` except for internal logic, and always re-wrap with the correct span. All lists are `Vec<WithSpan<Expr>>`.
//!
//! ## DEPRECATION NOTICE
//!
//! The legacy macroexpander API (functions operating on bare `Expr`, e.g., `expand_recursive`, `expand`, etc.) is deprecated.
//! All macroexpander logic, macro functions, and recursive expansion must operate on `WithSpan<Expr>`. Never unwrap to a bare `Expr` except for internal logic, and always re-wrap with the correct span. All lists are `Vec<WithSpan<Expr>>`.
//!
//! Use only the canonical API: `MacroExpander` and `SutraMacroExpander` trait, which operate on `WithSpan<Expr>`.
//!
//! The legacy API will be removed in a future release. See the architecture docs for details.

use crate::ast::{Expr, WithSpan};
use crate::error::{SutraError, SutraErrorKind};
use serde::de::VariantAccess;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;

/// Represents a single step in the macro expansion trace.
#[derive(Debug, Clone)]
pub struct TraceStep {
    /// A description of what happened in this step, e.g., "Expanding macro 'is?'".
    pub description: String,
    /// The state of the AST after this step's transformation.
    pub ast: Expr,
}

// A macro function is a native Rust function that transforms an AST.
pub type MacroFn = fn(
    &crate::ast::WithSpan<crate::ast::Expr>,
) -> Result<crate::ast::WithSpan<crate::ast::Expr>, crate::error::SutraError>;

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

// Custom Serialize/Deserialize for MacroDef
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

impl<'de> Deserialize<'de> for MacroDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Fn,
            Template,
        }

        struct MacroDefVisitor;
        impl<'de> serde::de::Visitor<'de> for MacroDefVisitor {
            type Value = MacroDef;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("enum MacroDef")
            }
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                let (field, variant) = data.variant::<String>()?;
                match field.as_str() {
                    "Template" => {
                        let tmpl = variant.newtype_variant::<MacroTemplate>()?;
                        Ok(MacroDef::Template(tmpl))
                    }
                    "Fn" => {
                        // Cannot deserialize native function pointers; return error or skip.
                        Err(serde::de::Error::custom(
                            "Cannot deserialize MacroDef::Fn variant",
                        ))
                    }
                    _ => Err(serde::de::Error::unknown_variant(&field, &[])),
                }
            }
        }
        deserializer.deserialize_enum("MacroDef", &["Fn", "Template"], MacroDefVisitor)
    }
}

/// Macro registry for built-in and template macros.
///
/// Example usage:
/// let mut registry = sutra::macros::MacroRegistry::default();
/// // Add a built-in macro (see tests/macro_expansion_tests.rs for real tests)
/// // registry.macros.insert("inc".to_string(), MacroDef::Fn(|_| unimplemented!()));
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: std::collections::HashMap<String, MacroDef>,
}

/// Macro context for macroexpansion, including registry and hygiene scope.
///
/// Example usage:
/// let context = sutra::macros::SutraMacroContext { registry: sutra::macros::MacroRegistry::default(), hygiene_scope: None };
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutraMacroContext {
    /// Macro registry (built-in and template macros).
    pub registry: MacroRegistry,
    /// Hygiene scope for macro expansion (optional, for future extensibility).
    pub hygiene_scope: Option<String>,
    // Extend as needed for user macros, environment, etc.
}

impl SutraMacroContext {
    /// Looks up a macro by name in the registry.
    ///
    /// Example usage:
    /// let mut registry = sutra::macros::MacroRegistry::default();
    /// // registry.macros.insert("foo".to_string(), MacroDef::Fn(|_| unimplemented!()));
    /// let context = sutra::macros::SutraMacroContext { registry, hygiene_scope: None };
    /// // assert!(context.get_macro("foo").is_some());
    pub fn get_macro(&self, name: &str) -> Option<&MacroDef> {
        self.registry.macros.get(name)
    }
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new macro with the given name.
    pub fn register(&mut self, name: &str, func: MacroFn) {
        self.macros.insert(name.to_string(), MacroDef::Fn(func));
    }

    // Helper: Check arity for macro template call
    fn check_arity(args_len: usize, params: &crate::ast::ParamList, span: &crate::ast::Span) -> Result<(), SutraError> {
        if args_len < params.required.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects at least {} arguments, but got {}.",
                    params.required.len(),
                    args_len
                )),
                span: Some(span.clone()),
            });
        }
        if params.rest.is_none() && args_len > params.required.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects exactly {} arguments, but got {}.",
                    params.required.len(),
                    args_len
                )),
                span: Some(span.clone()),
            });
        }
        Ok(())
    }

    // Helper: Bind macro parameters to arguments
    fn bind_macro_params(
        args: &[WithSpan<Expr>],
        params: &crate::ast::ParamList,
        expr_span: &crate::ast::Span,
    ) -> HashMap<String, WithSpan<Expr>> {
        let mut bindings = HashMap::new();
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

    fn expand_template(
        &self,
        template: &MacroTemplate,
        expr: &WithSpan<Expr>,
        _depth: usize,
    ) -> Result<WithSpan<Expr>, SutraError> {
        let (items, span) = match &expr.value {
            Expr::List(items, span) => (items, span),
            _ => {
                return Err(SutraError {
                    kind: SutraErrorKind::Macro(
                        "Template macro must be called as a list.".to_string(),
                    ),
                    span: Some(expr.span.clone()),
                });
            }
        };
        let args = &items[1..];
        // Arity check
        Self::check_arity(args.len(), &template.params, span)?;
        // Bind parameters
        let bindings = Self::bind_macro_params(args, &template.params, span);
        // Substitute parameters in the macro body
        let substituted_body = self.substitute(&template.body, &bindings)?;
        Ok(substituted_body)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn substitute(
        &self,
        expr: &WithSpan<Expr>,
        bindings: &HashMap<String, WithSpan<Expr>>,
    ) -> Result<WithSpan<Expr>, SutraError> {
        match &expr.value {
            Expr::Symbol(name, span) => {
                if let Some(bound_expr) = bindings.get(name) {
                    Ok(bound_expr.clone())
                } else {
                    Ok(WithSpan {
                        value: Expr::Symbol(name.clone(), span.clone()),
                        span: expr.span.clone(),
                    })
                }
            }
            Expr::Quote(inner, span) => {
                let new_inner = self.substitute(inner, bindings)?;
                Ok(WithSpan {
                    value: Expr::Quote(Box::new(new_inner), span.clone()),
                    span: expr.span.clone(),
                })
            }
            Expr::List(items, _span) => {
                let new_items = items
                    .iter()
                    .map(|item| self.substitute(item, bindings))
                    .collect::<Result<Vec<_>, _>>()?;
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
                let new_condition = self.substitute(condition, bindings)?;
                let new_then = self.substitute(then_branch, bindings)?;
                let new_else = self.substitute(else_branch, bindings)?;
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
            // Literals and paths are returned as-is
            _ => Ok(expr.clone()),
        }
    }

    /// Computes a SHA256 hash of all macro names and their source/expansion forms, sorted deterministically.
    pub fn hash(&self) -> String {
        let mut entries: Vec<(String, String)> = self
            .macros
            .iter()
            .map(|(name, def)| {
                let def_str = match def {
                    MacroDef::Template(template) => {
                        // Serialize params, variadic_param, and body in a stable way
                        let mut s = String::new();
                        s.push_str(&format!("params:{:?};", template.params));
                        s.push_str(&format!("variadic:{:?};", template.params.rest));
                        s.push_str(&format!("body:{};", template.body.value.pretty()));
                        s
                    }
                    MacroDef::Fn(_) => "native_fn".to_string(),
                };
                (name.clone(), def_str)
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut hasher = Sha256::new();
        for (name, def_str) in entries {
            hasher.update(name.as_bytes());
            hasher.update(def_str.as_bytes());
        }
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for duplicate parameters.
    pub fn new(
        params: crate::ast::ParamList,
        body: Box<WithSpan<Expr>>,
    ) -> Result<Self, SutraError> {
        // Check for duplicate parameter names
        let mut all_names = params.required.clone();
        if let Some(var) = &params.rest {
            all_names.push(var.clone());
        }
        let mut seen = std::collections::HashSet::new();
        for name in &all_names {
            if !seen.insert(name) {
                return Err(SutraError {
                    kind: SutraErrorKind::Macro(format!(
                        "Duplicate parameter name '{}' in macro definition.",
                        name
                    )),
                    span: Some(params.span.clone()),
                });
            }
        }
        Ok(MacroTemplate { params, body })
    }
}

/// Parses Sutra macro definitions from a source string.
///
/// This function scans the provided Sutra source code for macro definitions of the form `(define (name ...) body)`.
/// It returns a vector of pairs, where each pair contains the macro's name and its corresponding `MacroTemplate`.
///
/// # Arguments
/// * `source` - A string slice containing Sutra source code.
///
/// # Returns
/// * `Ok(Vec<(String, MacroTemplate)>)` - A vector of (macro name, macro template) pairs if parsing succeeds.
/// * `Err(SutraError)` - If the source contains invalid macro forms or duplicate macro names.
///
/// Note: This function is an internal pipeline component. For robust testing, see integration and unit tests in the test suite.
pub fn parse_macros_from_source(source: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    use crate::parser;

    let exprs = parser::parse(source)?;
    let mut macros = Vec::new();
    let mut names_seen = std::collections::HashSet::new();

    for expr in exprs {
        if let Some((macro_name, template)) = try_parse_macro_form(&expr, &mut names_seen)? {
            macros.push((macro_name, template));
        }
    }
    Ok(macros)
}

/// Attempts to parse a macro definition from an expression.
/// Returns Ok(Some((name, template))) if the expr is a valid macro form, Ok(None) otherwise.
fn try_parse_macro_form(expr: &crate::ast::WithSpan<crate::ast::Expr>, names_seen: &mut std::collections::HashSet<String>) -> Result<Option<(String, MacroTemplate)>, SutraError> {
    // Only process (define (name ...) body) forms
    let items = match &expr.value {
        Expr::List(items, _) => items,
        _ => return Ok(None),
    };
    if items.len() != 3 {
        return Ok(None);
    }
    match &items[0].value {
        Expr::Symbol(def, _) if def == "define" => {},
        _ => return Ok(None),
    }
    // Parse parameter list
    let param_list = match &items[1].value {
        Expr::ParamList(param_list) => param_list,
        _ => {
            return Err(SutraError {
                kind: SutraErrorKind::Macro("Macro parameter list must be a ParamList.".to_string()),
                span: Some(items[1].span.clone()),
            });
        }
    };
    let macro_name = extract_macro_name(param_list)?;
    if !names_seen.insert(macro_name.clone()) {
        return Err(SutraError {
            kind: SutraErrorKind::Macro(format!("Duplicate macro name '{}'.", macro_name)),
            span: Some(param_list.span.clone()),
        });
    }
    let params = crate::ast::ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span.clone(),
    };
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok(Some((macro_name, template)))
}

/// Extracts the macro name from a parameter list, or returns an error if missing.
fn extract_macro_name(param_list: &crate::ast::ParamList) -> Result<String, SutraError> {
    if let Some(name) = param_list.required.first() {
        Ok(name.clone())
    } else {
        Err(SutraError {
            kind: SutraErrorKind::Macro("Macro name must be the first element of the parameter list.".to_string()),
            span: Some(param_list.span.clone()),
        })
    }
}

/// Thin wrapper: loads macro definitions from a file.
pub fn load_macros_from_file(path: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path).map_err(|e| SutraError {
        kind: SutraErrorKind::Io(e.to_string()),
        span: None,
    })?;
    parse_macros_from_source(&source)
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

/// Canonical macroexpander trait for the modular pipeline.
pub trait SutraMacroExpander {
    fn expand_macros(
        &self,
        ast: crate::ast::WithSpan<crate::ast::Expr>,
        context: &SutraMacroContext,
    ) -> Result<crate::ast::WithSpan<crate::ast::Expr>, SutraMacroError>;
}

/// Macroexpander for the modular pipeline (Sprint 4).
pub struct MacroExpander {
    pub max_recursion: usize,
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self { max_recursion: 32 }
    }
}

impl SutraMacroExpander for MacroExpander {
    fn expand_macros(
        &self,
        ast: crate::ast::WithSpan<crate::ast::Expr>,
        context: &SutraMacroContext,
    ) -> Result<crate::ast::WithSpan<crate::ast::Expr>, SutraMacroError> {
        expand_macros_rec(&ast, context, 0, self.max_recursion)
    }
}

// Helper: Try to expand a macro call at the root of this AST node.
fn try_expand_macro(
    ast: &WithSpan<Expr>,
    context: &SutraMacroContext,
) -> Option<Result<WithSpan<Expr>, SutraMacroError>> {
    let items = match &ast.value {
        Expr::List(items, _) => items,
        _ => return None,
    };
    if items.is_empty() {
        return None;
    }
    let macro_name = match &items[0].value {
        Expr::Symbol(s, _) => s,
        _ => return None,
    };
    let macro_def = context.registry.macros.get(macro_name)?;
    let expanded = match macro_def {
        MacroDef::Fn(func) => func(ast).map_err(|e| SutraMacroError::Expansion {
            span: ast.span.clone(),
            macro_name: macro_name.clone(),
            message: e.to_string(),
        }),
        MacroDef::Template(template) => context.registry.expand_template(template, ast, 0).map_err(|e| SutraMacroError::Expansion {
            span: ast.span.clone(),
            macro_name: macro_name.clone(),
            message: e.to_string(),
        }),
    };
    Some(expanded)
}

fn expand_macros_rec(
    ast: &WithSpan<Expr>,
    context: &SutraMacroContext,
    depth: usize,
    max_depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    if depth > max_depth {
        return Err(SutraMacroError::RecursionLimit {
            span: ast.span.clone(),
            macro_name: "<unknown>".to_string(),
        });
    }
    if let Some(result) = try_expand_macro(ast, context) {
        let expanded = result?;
        return expand_macros_rec(&expanded, context, depth + 1, max_depth);
    }
    match &ast.value {
        Expr::List(items, _) => expand_list(items, ast, context, depth, max_depth),
        Expr::If { condition, then_branch, else_branch, span } =>
            expand_if(condition, then_branch, else_branch, span, ast, context, depth, max_depth),
        _ => Ok(ast.clone()),
    }
}

fn expand_list(
    items: &Vec<WithSpan<Expr>>,
    ast: &WithSpan<Expr>,
    context: &SutraMacroContext,
    depth: usize,
    max_depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    let expanded_items = items
        .iter()
        .map(|item| expand_macros_rec(item, context, depth + 1, max_depth))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WithSpan {
        value: Expr::List(expanded_items, ast.span.clone()),
        span: ast.span.clone(),
    })
}

fn expand_if(
    condition: &Box<WithSpan<Expr>>,
    then_branch: &Box<WithSpan<Expr>>,
    else_branch: &Box<WithSpan<Expr>>,
    span: &crate::ast::Span,
    ast: &WithSpan<Expr>,
    context: &SutraMacroContext,
    depth: usize,
    max_depth: usize,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    let cond = expand_macros_rec(condition, context, depth, max_depth)?;
    let then_b = expand_macros_rec(then_branch, context, depth, max_depth)?;
    let else_b = expand_macros_rec(else_branch, context, depth, max_depth)?;
    Ok(WithSpan {
        value: Expr::If {
            condition: Box::new(cond),
            then_branch: Box::new(then_b),
            else_branch: Box::new(else_b),
            span: span.clone(),
        },
        span: ast.span.clone(),
    })
}
