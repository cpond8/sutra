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
use crate::macros_std;
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::VariantAccess;
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
pub type MacroFn = fn(&crate::ast::WithSpan<crate::ast::Expr>) -> Result<crate::ast::WithSpan<crate::ast::Expr>, crate::error::SutraError>;

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
        enum Field { Fn, Template }

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
                        Err(serde::de::Error::custom("Cannot deserialize MacroDef::Fn variant"))
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
/// # Example
/// ```rust
/// use crate::macros::{MacroRegistry, MacroDef};
/// let mut registry = MacroRegistry::default();
/// // Add a built-in macro
/// registry.macros.insert("inc".to_string(), MacroDef::Fn(|_| unimplemented!()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: std::collections::HashMap<String, MacroDef>,
}

/// Macro context for macroexpansion, including registry and hygiene scope.
///
/// # Example
/// ```rust
/// use crate::macros::{MacroRegistry, SutraMacroContext};
/// let context = SutraMacroContext { registry: MacroRegistry::default(), hygiene_scope: None };
/// ```
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
    /// # Example
    /// ```rust
    /// use crate::macros::{MacroRegistry, SutraMacroContext, MacroDef};
    /// let mut registry = MacroRegistry::default();
    /// registry.macros.insert("foo".to_string(), MacroDef::Fn(|_| unimplemented!()));
    /// let context = SutraMacroContext { registry, hygiene_scope: None };
    /// assert!(context.get_macro("foo").is_some());
    /// ```
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

    fn expand_template(
        &self,
        template: &MacroTemplate,
        expr: &WithSpan<Expr>,
        depth: usize,
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

        // Arity check: too few arguments
        if args.len() < template.params.required.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects at least {} arguments, but got {}.",
                    template.params.required.len(),
                    args.len()
                )),
                span: Some(span.clone()),
            });
        }

        // Too many arguments for non-variadic macro
        if template.params.rest.is_none() && args.len() > template.params.required.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects exactly {} arguments, but got {}.",
                    template.params.required.len(),
                    args.len()
                )),
                span: Some(span.clone()),
            });
        }

        // Bind fixed parameters positionally
        let mut bindings = HashMap::new();
        for (i, param_name) in template.params.required.iter().enumerate() {
            bindings.insert(param_name.clone(), args[i].clone());
        }

        // Bind variadic parameter (if present) to a list of remaining args (may be empty)
        if let Some(variadic_name) = &template.params.rest {
            let rest_args = if args.len() > template.params.required.len() {
                args[template.params.required.len()..].to_vec()
            } else {
                vec![]
            };
            bindings.insert(
                variadic_name.clone(),
                WithSpan {
                    value: Expr::List(rest_args, span.clone()),
                    span: span.clone(),
                },
            );
        }

        // TODO: Consider moving arity/positional validation to MacroTemplate::new if possible.

        let substituted_body = self.substitute(&template.body, &bindings)?;
        Ok(substituted_body)
    }

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
                    Ok(WithSpan { value: Expr::Symbol(name.clone(), span.clone()), span: expr.span.clone() })
                }
            }
            Expr::Quote(inner, span) => {
                let new_inner = self.substitute(inner, bindings)?;
                Ok(WithSpan { value: Expr::Quote(Box::new(new_inner), span.clone()), span: expr.span.clone() })
            }
            Expr::List(items, span) => {
                let new_items = items
                    .iter()
                    .map(|item| self.substitute(item, bindings))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(WithSpan { value: Expr::List(new_items, span.clone()), span: expr.span.clone() })
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

    /// The recursive implementation for `macroexpand_trace`.
    fn trace_recursive(
        &self,
        expr: &WithSpan<Expr>,
        trace: &mut Vec<TraceStep>,
        depth: usize,
    ) -> Result<WithSpan<Expr>, SutraError> {
        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            return Err(SutraError {
                kind: SutraErrorKind::Macro("Macro expansion depth limit exceeded.".to_string()),
                span: Some(expr.span.clone()),
            });
        }

        if let Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } = &expr.value
        {
            let expanded_condition = self.trace_recursive(condition, trace, depth + 1)?;
            let expanded_then = self.trace_recursive(then_branch, trace, depth + 1)?;
            let expanded_else = self.trace_recursive(else_branch, trace, depth + 1)?;
            return Ok(WithSpan {
                value: Expr::If {
                    condition: Box::new(expanded_condition),
                    then_branch: Box::new(expanded_then),
                    else_branch: Box::new(expanded_else),
                    span: span.clone(),
                },
                span: expr.span.clone(),
            });
        }

        let (items, span) = match &expr.value {
            Expr::List(items, span) => (items, span),
            Expr::Quote(inner, span) => {
                // Do not expand inside quotes; return as-is
                return Ok(WithSpan {
                    value: Expr::Quote(inner.clone(), span.clone()),
                    span: expr.span.clone(),
                });
            }
            _ => return Ok(expr.clone()),
        };

        if items.is_empty() {
            return Ok(expr.clone());
        }

        if let Some(WithSpan { value: Expr::Symbol(s, _), .. }) = items.get(0) {
            if let Some(macro_def) = self.macros.get(s) {
                let expanded = match macro_def {
                    MacroDef::Fn(func) => func(expr)?,
                    MacroDef::Template(template) => self.expand_template(template, expr, depth)?,
                };
                trace.push(TraceStep {
                    description: format!("Expanding macro `{}`", s),
                    ast: expanded.value.clone(),
                });
                return self.trace_recursive(&expanded, trace, depth + 1);
            }
        }

        let expanded_items = items
            .iter()
            .map(|item| self.trace_recursive(item, trace, depth + 1))
            .collect::<Result<Vec<WithSpan<Expr>>, _>>()?;

        Ok(WithSpan {
            value: Expr::List(expanded_items, span.clone()),
            span: expr.span.clone(),
        })
    }

    /// Computes a SHA256 hash of all macro names and their source/expansion forms, sorted deterministically.
    pub fn hash(&self) -> String {
        let mut entries: Vec<(String, String)> = self.macros.iter().map(|(name, def)| {
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
        }).collect();
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
        Ok(MacroTemplate {
            params,
            body,
        })
    }
}

/// Pure loader: parses macro definitions from a Sutra source string.
pub fn parse_macros_from_source(source: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    use crate::parser;
    use crate::ast::Expr;
    use std::collections::{HashMap, HashSet};

    let exprs = parser::parse(source)?;
    let mut macros = Vec::new();
    let mut names_seen = HashSet::new();

    for expr in exprs {
        // Only process (define (name ...) body) forms
        if let Expr::List(items, span) = &expr.value {
            if items.len() == 3 {
                if let Expr::Symbol(def, _) = &items[0].value {
                    if def == "define" {
                        // Parse parameter list
                        match &items[1].value {
                            Expr::ParamList(param_list) => {
                                let macro_name = if let Some(name) = param_list.required.first() {
                                    name.clone()
                                } else {
                                    return Err(SutraError {
                                        kind: SutraErrorKind::Macro("Macro name must be the first element of the parameter list.".to_string()),
                                        span: Some(param_list.span.clone()),
                                    });
                                };
                                if !names_seen.insert(macro_name.clone()) {
                                    return Err(SutraError {
                                        kind: SutraErrorKind::Macro(format!("Duplicate macro name '{}'.", macro_name)),
                                        span: Some(param_list.span.clone()),
                                    });
                                }
                                // The rest of required are the macro parameters
                                let params = crate::ast::ParamList {
                                    required: param_list.required[1..].to_vec(),
                                    rest: param_list.rest.clone(),
                                    span: param_list.span.clone(),
                                };
                                let template = MacroTemplate::new(
                                    params,
                                    Box::new(items[2].clone()),
                                )?;
                                macros.push((macro_name, template));
                            }
                            _ => {
                                return Err(SutraError {
                                    kind: SutraErrorKind::Macro("Macro parameter list must be a ParamList.".to_string()),
                                    span: Some(items[1].span.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(macros)
}

/// Thin wrapper: loads macro definitions from a file.
pub fn load_macros_from_file(path: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path)
        .map_err(|e| SutraError { kind: SutraErrorKind::Io(e.to_string()), span: None })?;
    parse_macros_from_source(&source)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraMacroError {
    Expansion { span: crate::ast::Span, macro_name: String, message: String },
    RecursionLimit { span: crate::ast::Span, macro_name: String },
    // ...
}

/// Canonical macroexpander trait for the modular pipeline.
pub trait SutraMacroExpander {
    fn expand_macros(&self, ast: crate::ast::WithSpan<crate::ast::Expr>, context: &SutraMacroContext) -> Result<crate::ast::WithSpan<crate::ast::Expr>, SutraMacroError>;
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
    fn expand_macros(&self, ast: crate::ast::WithSpan<crate::ast::Expr>, context: &SutraMacroContext) -> Result<crate::ast::WithSpan<crate::ast::Expr>, SutraMacroError> {
        expand_macros_rec(&ast, context, 0, self.max_recursion)
    }
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
    match &ast.value {
        Expr::List(items, span) => {
            if items.is_empty() {
                return Ok(ast.clone());
            }
            if let Some(WithSpan { value: Expr::Symbol(s, _), .. }) = items.get(0) {
                if let Some(macro_def) = context.registry.macros.get(s) {
                    let expanded = match macro_def {
                        MacroDef::Fn(func) => func(ast).map_err(|e| SutraMacroError::Expansion {
                            span: ast.span.clone(),
                            macro_name: s.clone(),
                            message: e.to_string(),
                        })?,
                        MacroDef::Template(template) => context.registry.expand_template(template, ast, depth).map_err(|e| SutraMacroError::Expansion {
                            span: ast.span.clone(),
                            macro_name: s.clone(),
                            message: e.to_string(),
                        })?,
                    };
                    return expand_macros_rec(&expanded, context, depth + 1, max_depth);
                }
            }
            let expanded_items = items
                .iter()
                .map(|item| expand_macros_rec(item, context, depth + 1, max_depth))
                .collect::<Result<Vec<WithSpan<Expr>>, _>>()?;
            Ok(WithSpan {
                value: Expr::List(expanded_items, span.clone()),
                span: ast.span.clone(),
            })
        }
        Expr::If { condition, then_branch, else_branch, span } => {
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
        _ => Ok(ast.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, WithSpan, Span};
    use std::collections::HashMap;

    fn make_symbol(name: &str, span: Span) -> WithSpan<Expr> {
        WithSpan { value: Expr::Symbol(name.to_string(), span.clone()), span }
    }
    fn make_list(items: Vec<WithSpan<Expr>>, span: Span) -> WithSpan<Expr> {
        WithSpan { value: Expr::List(items, span.clone()), span }
    }

    #[test]
    fn expands_builtin_macro() {
        fn inc_macro(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, crate::error::SutraError> {
            if let Expr::List(items, span) = &expr.value {
                if items.len() == 2 {
                    Ok(WithSpan {
                        value: Expr::List(
                            vec![
                                WithSpan { value: Expr::Symbol("+".to_string(), span.clone()), span: span.clone() },
                                items[1].clone(),
                                WithSpan { value: Expr::Number(1.0, span.clone()), span: span.clone() },
                            ],
                            span.clone(),
                        ),
                        span: span.clone(),
                    })
                } else {
                    Err(crate::error::SutraError {
                        kind: crate::error::SutraErrorKind::Macro("inc expects 1 arg".to_string()),
                        span: Some(span.clone()),
                    })
                }
            } else {
                Err(crate::error::SutraError {
                    kind: crate::error::SutraErrorKind::Macro("inc expects list".to_string()),
                    span: None,
                })
            }
        }
        let mut registry = MacroRegistry::default();
        registry.macros.insert("inc".to_string(), MacroDef::Fn(inc_macro));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let span = Span { start: 0, end: 7 };
        let ast = make_list(
            vec![make_symbol("inc", span.clone()), make_symbol("x", span.clone())],
            span.clone(),
        );
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast, &context).unwrap();
        if let Expr::List(items, _) = &expanded.value {
            assert_eq!(items.len(), 3);
            assert!(matches!(&items[0].value, Expr::Symbol(s, _) if s == "+"));
            assert!(matches!(&items[1].value, Expr::Symbol(s, _) if s == "x"));
            assert!(matches!(&items[2].value, Expr::Number(n, _) if *n == 1.0));
        } else {
            panic!("Expected expanded macro to be a list");
        }
    }

    #[test]
    fn recursion_limit() {
        fn self_macro(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, crate::error::SutraError> {
            Ok(expr.clone())
        }
        let mut registry = MacroRegistry::default();
        registry.macros.insert("self".to_string(), MacroDef::Fn(self_macro));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let span = Span { start: 0, end: 6 };
        let ast = make_list(vec![make_symbol("self", span.clone())], span.clone());
        let expander = MacroExpander { max_recursion: 4 };
        let err = expander.expand_macros(ast, &context).unwrap_err();
        match err {
            SutraMacroError::RecursionLimit { .. } => {}
            _ => panic!("Expected recursion limit error"),
        }
    }

    #[test]
    fn undefined_macro_returns_input() {
        let registry = MacroRegistry::default();
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let span = Span { start: 0, end: 5 };
        let ast = make_list(vec![make_symbol("foo", span.clone())], span.clone());
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast, &context).unwrap();
        assert!(matches!(&expanded.value, Expr::List(_, _)));
    }
}
