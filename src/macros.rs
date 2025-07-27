//! # Clean Macro Expansion System
//!
//! Core responsibility: Transform user-friendly syntax into canonical AST forms.
//!
//! This module performs three essential functions:
//! 1. Template substitution - Replace parameters in macro bodies with arguments
//! 2. Path canonicalization - Convert various path syntaxes to Expr::Path nodes
//! 3. Built-in macro expansion - Transform syntax sugar into canonical forms

use std::collections::HashMap;

use crate::prelude::*;
use crate::{
    errors::{to_source_span, ErrorKind, ErrorReporting, SourceContext, SutraError},
    syntax::{parser, ParamList},
    validation::ValidationContext,
};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum recursion depth for macro expansion to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 100;

/// Expected arity for macro definitions (define name params body)
const MACRO_DEFINITION_ARITY: usize = 3;

// ============================================================================
// CORE TYPES
// ============================================================================

/// A native Rust function that transforms an AST node
pub type MacroFunction = fn(&AstNode) -> Result<AstNode, SutraError>;

/// A template macro for parameter substitution
#[derive(Debug, Clone, PartialEq)]
pub struct MacroTemplate {
    pub params: ParamList,
    pub body: Box<AstNode>,
}

impl MacroTemplate {
    pub fn new(params: ParamList, body: Box<AstNode>) -> Result<Self, SutraError> {
        Ok(MacroTemplate { params, body })
    }
}

/// A macro definition - either a native function or template
#[derive(Debug, Clone)]
pub enum MacroDefinition {
    /// Native Rust function macro
    Function(MacroFunction),
    /// A declarative template macro
    Template(MacroTemplate),
}

/// The complete macro expansion system
#[derive(Debug, Clone)]
pub struct MacroSystem {
    macros: HashMap<String, MacroDefinition>,
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

impl MacroSystem {
    /// Create a new macro system with built-in macros registered
    pub fn new() -> Self {
        let mut system = Self {
            macros: HashMap::new(),
        };
        register_builtins(&mut system);
        system
    }

    /// Expand all macros in an AST node
    pub fn expand(&self, ast: AstNode) -> Result<AstNode, SutraError> {
        expand_recursive(self, ast, 0)
    }

    /// Load and register macros from source code
    pub fn load_from_source(&mut self, source: &str) -> Result<(), SutraError> {
        let source_ctx = SourceContext::from_file("macro_source", source);
        let exprs = parser::parse(source, source_ctx)?;

        for expr in exprs {
            if let Some((name, def)) = parse_macro_definition_internal(&expr)? {
                self.macros.insert(name, def);
            }
        }
        Ok(())
    }

    /// Register a macro
    pub fn register(&mut self, name: String, definition: MacroDefinition) {
        self.macros.insert(name, definition);
    }

    /// Get all macro names
    pub fn macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }

    /// Check if a macro exists
    pub fn has_macro(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Get a macro definition
    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }
}

// ============================================================================
// CORE EXPANSION LOGIC
// ============================================================================

/// Recursively expand macros with depth checking
fn expand_recursive(
    system: &MacroSystem,
    node: AstNode,
    depth: usize,
) -> Result<AstNode, SutraError> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(create_error(
            ErrorKind::RecursionLimit,
            "macro_expansion",
            "recursion_limit",
            node.span,
        ));
    }

    // Check if this is a macro call
    let Expr::List(items, _) = &*node.value else {
        return expand_subforms(system, node, depth);
    };

    let Some(first) = items.first() else {
        return expand_subforms(system, node, depth);
    };

    let Expr::Symbol(name, _) = &*first.value else {
        return expand_subforms(system, node, depth);
    };

    if let Some(macro_def) = system.macros.get(name) {
        let expanded = apply_macro(&node, macro_def)?;
        return expand_recursive(system, expanded, depth + 1);
    }

    expand_subforms(system, node, depth)
}

/// Helper to expand subforms without macro call detection
fn expand_subforms(
    system: &MacroSystem,
    node: AstNode,
    depth: usize,
) -> Result<AstNode, SutraError> {
    match &*node.value {
        Expr::List(items, span) => {
            let mut expanded_items = Vec::new();
            for item in items {
                expanded_items.push(expand_recursive(system, item.clone(), depth)?);
            }
            Ok(Spanned {
                value: Expr::List(expanded_items, *span).into(),
                span: node.span,
            })
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let new_condition = expand_recursive(system, condition.as_ref().clone(), depth)?;
            let new_then = expand_recursive(system, then_branch.as_ref().clone(), depth)?;
            let new_else = expand_recursive(system, else_branch.as_ref().clone(), depth)?;
            Ok(Spanned {
                value: Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: Box::new(new_else),
                    span: *span,
                }
                .into(),
                span: node.span,
            })
        }
        Expr::Quote(inner, span) => {
            let new_inner = expand_recursive(system, inner.as_ref().clone(), depth)?;
            Ok(Spanned {
                value: Expr::Quote(Box::new(new_inner), *span).into(),
                span: node.span,
            })
        }
        Expr::Spread(inner) => {
            let new_inner = expand_recursive(system, inner.as_ref().clone(), depth)?;
            Ok(Spanned {
                value: Expr::Spread(Box::new(new_inner)).into(),
                span: node.span,
            })
        }
        _ => Ok(node),
    }
}

/// Apply a single macro to a call node
fn apply_macro(call: &AstNode, definition: &MacroDefinition) -> Result<AstNode, SutraError> {
    match definition {
        MacroDefinition::Function(f) => f(call),
        MacroDefinition::Template(template) => {
            let (args, _span) = extract_args_from_call(call)?;

            // Validate arity
            let required_count = template.params.required.len();
            let has_variadic = template.params.rest.is_some();

            if args.len() < required_count || (args.len() > required_count && !has_variadic) {
                let expected = if has_variadic {
                    format!("at least {}", required_count)
                } else {
                    required_count.to_string()
                };
                return Err(create_arity_error(&expected, args.len(), call.span));
            }

            // Bind parameters
            let mut bindings = HashMap::new();
            for (i, param) in template.params.required.iter().enumerate() {
                bindings.insert(param.clone(), args[i].clone());
            }

            // Handle variadic parameter
            if let Some(var_name) = &template.params.rest {
                let rest_args: Vec<AstNode> = if args.len() > required_count {
                    args[required_count..].to_vec()
                } else {
                    Vec::new()
                };

                // Create (list ...rest_args)
                let mut list_items = vec![Spanned {
                    value: Expr::Symbol("list".to_string(), call.span).into(),
                    span: call.span,
                }];
                list_items.extend(rest_args);

                let list_node = Spanned {
                    value: Expr::List(list_items, call.span).into(),
                    span: call.span,
                };
                bindings.insert(var_name.clone(), list_node);
            }

            substitute_template(&template.body, &bindings)
        }
    }
}

/// Substitute parameters in a template body
fn substitute_template(
    node: &AstNode,
    bindings: &HashMap<String, AstNode>,
) -> Result<AstNode, SutraError> {
    // Handle symbol substitution
    if let Expr::Symbol(name, _) = &*node.value {
        return Ok(bindings.get(name).cloned().unwrap_or_else(|| node.clone()));
    }

    // Handle list processing
    let Expr::List(items, span) = &*node.value else {
        // Not a list, return unchanged
        return Ok(node.clone());
    };

    let mut new_items = Vec::new();

    for item in items {
        // Handle spread operator
        let Expr::Spread(inner) = &*item.value else {
            // Not a spread, substitute normally
            new_items.push(substitute_template(item, bindings)?);
            continue;
        };

        // Process spread inner expression
        let substituted = substitute_template(inner, bindings)?;

        // Extract list elements from substituted result
        let Expr::List(elements, _) = &*substituted.value else {
            return Err(create_type_error(
                "list",
                substituted.value.type_name(),
                inner.span,
            ));
        };

        new_items.extend(elements.clone());
    }

    Ok(Spanned {
        value: Expr::List(new_items, *span).into(),
        span: node.span,
    })
}

// ============================================================================
// PARSING AND UTILITIES
// ============================================================================

/// Parse a macro definition from AST (internal version)
fn parse_macro_definition_internal(
    expr: &AstNode,
) -> Result<Option<(String, MacroDefinition)>, SutraError> {
    let Expr::List(items, _) = &*expr.value else {
        return Ok(None);
    };

    if items.len() != MACRO_DEFINITION_ARITY {
        return Ok(None);
    }

    let Expr::Symbol(def_name, _) = &*items[0].value else {
        return Ok(None);
    };
    if def_name != "define" {
        return Ok(None);
    }

    let Expr::ParamList(param_list) = &*items[1].value else {
        return Ok(None);
    };

    if param_list.required.is_empty() {
        return Err(create_error(
            ErrorKind::MissingElement {
                element: "macro name".to_string(),
            },
            "macro_definition",
            "empty_params",
            items[1].span,
        ));
    }

    let macro_name = param_list.required[0].clone();
    let params = param_list.required[1..].to_vec();
    let variadic = param_list.rest.clone();
    let body = items[2].clone();

    let template = MacroTemplate {
        params: ParamList {
            required: params,
            rest: variadic,
            span: param_list.span,
        },
        body: Box::new(body),
    };

    Ok(Some((macro_name, MacroDefinition::Template(template))))
}

/// Extract arguments from a macro call
fn extract_args_from_call(call: &AstNode) -> Result<(Vec<AstNode>, Span), SutraError> {
    let Expr::List(items, span) = &*call.value else {
        return Err(create_error(
            ErrorKind::MalformedConstruct {
                construct: "macro call".to_string(),
            },
            "macro_expansion",
            "invalid_call",
            call.span,
        ));
    };

    if items.is_empty() {
        return Err(create_error(
            ErrorKind::EmptyExpression,
            "macro_expansion",
            "empty_call",
            *span,
        ));
    }

    Ok((items[1..].to_vec(), *span))
}

// ============================================================================
// PATH CANONICALIZATION
// ============================================================================

// Note: Path canonicalization has been moved to atoms/world.rs
// This section is preserved for potential future template macro needs

// ============================================================================
// ERROR HELPERS
// ============================================================================

/// Create a consistent error with validation context
fn create_error(kind: ErrorKind, module: &str, context: &str, span: Span) -> SutraError {
    let ctx = ValidationContext::new(
        SourceContext::from_file(module, context),
        "macro_expansion".to_string(),
    );
    ctx.report(kind, to_source_span(span))
}

/// Create an arity mismatch error
fn create_arity_error(expected: &str, actual: usize, span: Span) -> SutraError {
    let ctx = ValidationContext::new(
        SourceContext::from_file("macro_expansion", "arity"),
        "macro_expansion".to_string(),
    );
    ctx.arity_mismatch(expected, actual, to_source_span(span))
}

/// Create a type mismatch error
fn create_type_error(expected: &str, actual: &str, span: Span) -> SutraError {
    let ctx = ValidationContext::new(
        SourceContext::from_file("macro_expansion", "type_error"),
        "macro_expansion".to_string(),
    );
    ctx.type_mismatch(expected, actual, to_source_span(span))
}

// ============================================================================
// BUILT-IN MACROS
// ============================================================================

/// Register all built-in macros
fn register_builtins(_system: &mut MacroSystem) {
    // Note: Most world state operations (set!, get, del!, exists?, inc!, dec!, add!, sub!, print)
    // are now implemented as direct atoms rather than macros.
    // Only template macros and complex transformations remain here.
}
