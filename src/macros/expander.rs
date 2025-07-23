//!
//! Manages template substitution, variadic forwarding, and recursive expansion,
//! ensuring proper error handling and recursion depth checking.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using miette-native error variants directly. See `src/errors.rs` for error types and usage rules.
//!
//! Example: let err = SutraError::ValidationGeneral { message: "Invalid macro expansion".to_string(), ... };
//!
//! All macro expansion errors (arity, recursion, substitution, etc.) use this system.
//!
//! ## Recursion and Expansion
//!
//! This module provides the core expansion engine, including recursion depth checks and trace recording. All errors related to recursion limits or invalid macro forms are reported using the canonical error system.

use std::collections::HashMap;

use crate::prelude::*;
use crate::{
    ast::ParamList,
    errors,
    macros::{
        check_arity, MacroDefinition, MacroExpansionContext, MacroExpansionResult,
        MacroExpansionStep, MacroTemplate, MAX_MACRO_RECURSION_DEPTH,
    },
    syntax::parser::to_source_span,
};

// =============================
// Type aliases to reduce verbosity
// =============================

/// Type alias for macro parameter bindings
type MacroBindings = HashMap<String, AstNode>;

// =============================
// Public API for macro expansion
// =============================

/// Expands all macro calls recursively within an AST node, tracing expansions.
/// This is the main entry point for macro expansion during evaluation.
pub fn expand_macros_recursively(
    ast: AstNode,
    env: &mut MacroExpansionContext,
) -> MacroExpansionResult {
    expand_macros_recursively_with_trace(ast, env, 0)
}

/// Expands a single macro call, handling both template and function macros.
/// Used by the evaluator to replace inline macro expansion logic.
pub fn expand_macro_call(
    macro_def: &MacroDefinition,
    call: &AstNode,
    env: &MacroExpansionContext,
    depth: usize,
) -> MacroExpansionResult {
    check_recursion_depth(depth, "macro", &call.span, env)?;

    match macro_def {
        MacroDefinition::Template(template) => expand_template(template, call, depth, env),
        MacroDefinition::Fn(func) => func(call),
    }
}

// =============================
// Template substitution
// =============================

/// Expands a macro template by substituting arguments into the template body.
/// Performs arity checks, parameter binding, and recursion depth validation.
pub fn expand_template(
    template: &MacroTemplate,
    call: &AstNode,
    depth: usize,
    env: &MacroExpansionContext,
) -> MacroExpansionResult {
    // Extract macro name and arguments from a macro call
    let (macro_name, args, span) = extract_macro_call_info(call)?;

    check_recursion_depth(depth, macro_name, span, env)?;
    check_arity(args.len(), &template.params, macro_name, span)?;

    let bindings = bind_macro_params(args, &template.params, span);
    substitute_template(&template.body, &bindings, env, depth + 1)
}

/// Extracts macro name and arguments from a macro call
fn extract_macro_call_info(call: &AstNode) -> Result<(&str, &[AstNode], &Span), SutraError> {
    let Expr::List(items, span) = &*call.value else {
        return Err(
            errors::runtime_general("Macro call must be a list expression", "macro call", format!("{:?}", call), to_source_span(call.span))
                .with_suggestion("Macros must be called using list syntax, like `(my-macro ...)`.")
        );
    };

    if items.is_empty() {
        return Err(
            errors::runtime_general("Macro call cannot be empty", "macro call", format!("{:?}", call), to_source_span(*span))
        );
    }

    let first = &items[0];
    let Expr::Symbol(macro_name, _) = &*first.value else {
        return Err(
            errors::runtime_general("Macro call head must be a symbol", "macro call", format!("{:?}", call), to_source_span(first.span))
                .with_suggestion("The first element of a macro call must be the macro's name.")
        );
    };

    Ok((macro_name, &items[1..], span))
}

/// Binds macro parameters to arguments for template substitution.
/// Handles both regular and variadic parameters with proper list construction.
pub fn bind_macro_params(args: &[AstNode], params: &ParamList, span: &Span) -> MacroBindings {
    let mut bindings = HashMap::new();

    // Bind regular parameters
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
        with_span(Expr::List(rest_args, *span), span),
    );

    bindings
}

/// Recursively substitutes macro parameters in template expressions.
/// Handles symbol substitution, list traversal, and variadic splicing.
pub fn substitute_template(
    expr: &AstNode,
    bindings: &MacroBindings,
    env: &MacroExpansionContext,
    depth: usize,
) -> MacroExpansionResult {
    check_recursion_depth(depth, "macro", &expr.span, env)?;

    let value = &*expr.value;

    // Handle symbol substitution
    if let Expr::Symbol(name, _) = value {
        return Ok(bindings.get(name).cloned().unwrap_or_else(|| expr.clone()));
    }

    // Handle list processing with spreads
    if let Expr::List(items, _) = value {
        let mut new_items = Vec::new();

        for item in items {
            // Handle spread expressions by expanding their elements
            if let Expr::Spread(inner) = &*item.value {
                let substituted = substitute_template(inner, bindings, env, depth + 1)?;
                let Expr::List(elements, _) = &*substituted.value else {
                    return Err(errors::type_mismatch(
                        "list",
                        substituted.type_name(),
                        env.source.name().to_string(),
                        env.source.inner().clone(),
                        to_source_span(inner.span),
                    )
                    .with_suggestion("Spread expressions (...) in macros must be bound to a list value."));
                };
                new_items.extend(elements.clone());
                continue;
            }

            // Handle regular expressions by substituting normally
            let substituted = substitute_template(item, bindings, env, depth + 1)?;
            new_items.push(substituted);
        }

        let span = items.first().unwrap().span;
        return Ok(with_span(Expr::List(new_items, span), &span));
    }

    // Handle other expression types
    match value {
        Expr::Quote(inner, span) => {
            // Don't substitute inside quotes - they should be literal
            Ok(with_span(Expr::Quote(inner.clone(), *span), &expr.span))
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let new_condition = substitute_template(condition, bindings, env, depth + 1)?;
            let new_then = substitute_template(then_branch, bindings, env, depth + 1)?;
            let new_else = substitute_template(else_branch, bindings, env, depth + 1)?;
            Ok(with_span(
                Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: Box::new(new_else),
                    span: *span,
                },
                &expr.span,
            ))
        }
        _ => Ok(expr.clone()),
    }
}

// =============================
// Macro expansion
// =============================

/// Recursively expands macros and records trace.
fn expand_macros_recursively_with_trace(
    node: AstNode,
    env: &mut MacroExpansionContext,
    depth: usize,
) -> MacroExpansionResult {
    if let Some(expanded) = try_expand_macro_once(&node, env, depth)? {
        return expand_macros_recursively_with_trace(expanded, env, depth + 1);
    }
    map_ast(node, &expand_macros_recursively_with_trace, env, depth)
}

/// Expands a macro call once, checking recursion depth.
fn try_expand_macro_once(
    node: &AstNode,
    env: &mut MacroExpansionContext,
    depth: usize,
) -> Result<Option<AstNode>, SutraError> {
    let Expr::List(items, _) = &*node.value else {
        return Ok(None);
    };

    let first = items.first().ok_or_else(|| {
        errors::runtime_general(
            "Unexpected empty list in macro expansion check",
            "macro expansion",
            format!("{:?}", node),
            to_source_span(node.span)
        )
    })?;

    let Expr::Symbol(macro_name, _) = &*first.value else {
        return Ok(None);
    };

    check_recursion_depth(depth, "macro", &node.span, env)?;

    let (provenance, macro_def) = match env.lookup_macro(macro_name) {
        Some((prov, def)) => (prov, def.clone()),
        None => return Ok(None),
    };

    let original_node = node.clone();
    let result = match &macro_def {
        MacroDefinition::Fn(func) => func(node),
        MacroDefinition::Template(template) => expand_template(template, node, depth, env),
    };

    if let Ok(expanded_node) = &result {
        env.trace.push(MacroExpansionStep {
            macro_name: macro_name.to_string(),
            provenance,
            input: original_node,
            output: expanded_node.clone(),
        });
    }

    result.map(Some)
}

// =============================
// AST traversal
// =============================

/// Recursively maps a function over AST nodes for macro expansion.
fn map_ast<F>(
    node: AstNode,
    f: &F,
    env: &mut MacroExpansionContext,
    depth: usize,
) -> MacroExpansionResult
where
    F: Fn(AstNode, &mut MacroExpansionContext, usize) -> MacroExpansionResult,
{
    match &*node.value {
        Expr::List(items, span) => {
            let new_items: Vec<_> = items
                .iter()
                .map(|item| f(item.clone(), env, depth + 1))
                .collect::<Result<_, _>>()?;

            Ok(with_span(Expr::List(new_items, *span), &node.span))
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let new_condition = f(condition.as_ref().clone(), env, depth + 1)?;
            let new_then_branch = f(then_branch.as_ref().clone(), env, depth + 1)?;
            let new_else_branch = f(else_branch.as_ref().clone(), env, depth + 1)?;

            Ok(with_span(
                Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then_branch),
                    else_branch: Box::new(new_else_branch),
                    span: *span,
                },
                &node.span,
            ))
        }
        Expr::Quote(inner, span) => {
            let new_inner = f(inner.as_ref().clone(), env, depth + 1)?;
            Ok(with_span(
                Expr::Quote(Box::new(new_inner), *span),
                &node.span,
            ))
        }
        Expr::Spread(inner) => {
            let new_inner = f(inner.as_ref().clone(), env, depth + 1)?;
            Ok(with_span(Expr::Spread(Box::new(new_inner)), &node.span))
        }
        // Atomic types (Symbol, Path, String, Number, Bool) and ParamList don't need traversal
        _ => Ok(node),
    }
}

// =============================
// Utilities
// =============================

/// Checks recursion depth and returns error if exceeded.
fn check_recursion_depth(
    depth: usize,
    macro_name: &str,
    span: &Span,
    env: &MacroExpansionContext,
) -> Result<(), SutraError> {
    if depth <= MAX_MACRO_RECURSION_DEPTH {
        return Ok(());
    }
    Err(errors::runtime_general(
        format!("Macro recursion limit of {} exceeded", MAX_MACRO_RECURSION_DEPTH),
        env.source.name().to_string(),
        env.source.inner().clone(),
        to_source_span(*span),
    )
    .with_suggestion(format!("Check for infinite recursion in macro '{}'", macro_name)))
}

/// Creates a Spanned wrapper with consistent span handling.
fn with_span(value: Expr, original_span: &Span) -> AstNode {
    Spanned {
        value: value.into(),
        span: *original_span,
    }
}
