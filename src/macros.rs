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

use crate::ast::{Expr, WithSpan};
use crate::error::{SutraError, SutraErrorKind};
use serde::de::VariantAccess;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;

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
    fn check_arity(
        args_len: usize,
        params: &crate::ast::ParamList,
        span: &crate::ast::Span,
    ) -> Result<(), SutraError> {
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
}

const MAX_MACRO_RECURSION_DEPTH: usize = 128;

pub fn expand_template(
    registry: &std::collections::HashMap<String, MacroDef>,
    template: &MacroTemplate,
    expr: &WithSpan<Expr>,
    depth: usize,
) -> Result<WithSpan<Expr>, SutraError> {
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(SutraError {
            kind: SutraErrorKind::Macro(format!(
                "Macro expansion recursion limit ({}) exceeded.",
                MAX_MACRO_RECURSION_DEPTH
            )),
            span: Some(expr.span.clone()),
        });
    }
    let (items, span) = match &expr.value {
        Expr::List(items, span) => (items, span),
        _ => {
            return Err(SutraError {
                kind: SutraErrorKind::Macro("Template macro must be called as a list.".to_string()),
                span: Some(expr.span.clone()),
            });
        }
    };
    let args = &items[1..];
    // Arity check
    MacroRegistry::check_arity(args.len(), &template.params, span)?;
    // Bind parameters
    let bindings = MacroRegistry::bind_macro_params(args, &template.params, span);
    // Substitute parameters in the macro body
    let substituted_body = substitute_template(registry, &template.body, &bindings)?;
    Ok(substituted_body)
}

pub fn substitute_template(
    _registry: &std::collections::HashMap<String, MacroDef>,
    expr: &WithSpan<Expr>,
    bindings: &std::collections::HashMap<String, WithSpan<Expr>>,
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
            let new_inner = substitute_template(_registry, inner, bindings)?;
            Ok(WithSpan {
                value: Expr::Quote(Box::new(new_inner), span.clone()),
                span: expr.span.clone(),
            })
        }
        Expr::List(items, _span) => {
            let new_items = items
                .iter()
                .map(|item| substitute_template(_registry, item, bindings))
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
            let new_condition = substitute_template(_registry, condition, bindings)?;
            let new_then = substitute_template(_registry, then_branch, bindings)?;
            let new_else = substitute_template(_registry, else_branch, bindings)?;
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
                Err(SutraError {
                    kind: SutraErrorKind::Macro(format!(
                        "Duplicate parameter name '{}' in macro definition.",
                        name
                    )),
                    span: Some(params.span.clone()),
                })?
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
fn try_parse_macro_form(
    expr: &crate::ast::WithSpan<crate::ast::Expr>,
    names_seen: &mut std::collections::HashSet<String>,
) -> Result<Option<(String, MacroTemplate)>, SutraError> {
    // Only process (define (name ...) body) forms
    let Expr::List(items, _) = &expr.value else { return Ok(None); };
    if items.len() != 3 {
        return Ok(None);
    }
    let Expr::Symbol(def, _) = &items[0].value else { return Ok(None); };
    if def != "define" {
        return Ok(None);
    }
    let Expr::ParamList(param_list) = &items[1].value else {
        Err(SutraError {
            kind: SutraErrorKind::Macro(
                "Macro parameter list must be a ParamList.".to_string(),
            ),
            span: Some(items[1].span.clone()),
        })?
    };
    let macro_name = extract_macro_name(param_list)?;
    if !names_seen.insert(macro_name.clone()) {
        Err(SutraError {
            kind: SutraErrorKind::Macro(format!("Duplicate macro name '{}'.", macro_name)),
            span: Some(param_list.span.clone()),
        })?
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
            kind: SutraErrorKind::Macro(
                "Macro name must be the first element of the parameter list.".to_string(),
            ),
            span: Some(param_list.span.clone()),
        })?
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
    pub user_macros: std::collections::HashMap<String, MacroDef>,
    pub core_macros: std::collections::HashMap<String, MacroDef>,
    pub trace: Vec<MacroExpansionStep>,
}

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

/// Attempts to expand a node as a macro. Returns Some((macro_name, provenance, expanded_node)) if expanded, else None.
/// Pure: no mutation, no trace, no recursion.
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
        let expanded = match macro_def {
            MacroDef::Fn(func) => func(node).map_err(|e| SutraMacroError::Expansion {
                span: node.span.clone(),
                macro_name: macro_name.clone(),
                message: e.to_string(),
            })?,
            MacroDef::Template(template) => {
                expand_template(&env.core_macros, template, node, depth + 1).map_err(|e| {
                    SutraMacroError::Expansion {
                        span: node.span.clone(),
                        macro_name: macro_name.clone(),
                        message: e.to_string(),
                    }
                })?
            }
        };
        return Ok(Some((macro_name.clone(), provenance, expanded)));
    }
    Ok(None)
}

/// Recursively applies a transformation function to a node and its children.
/// Used for macro expansion and other AST transformations.
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

/// Records a macro expansion step in the trace.
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

/// Recursively expands macros in the AST, recording each step in the trace.
/// This is the main entry point for macro expansion.
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

/// Public entry point for macro expansion.
pub fn expand_macros(
    ast: WithSpan<Expr>,
    env: &mut MacroEnv,
) -> Result<WithSpan<Expr>, SutraMacroError> {
    expand_macros_with_trace(ast, env, 0)
}
