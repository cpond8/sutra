//!
//! Manages template substitution, variadic forwarding, and recursive expansion,
//! ensuring proper error handling and recursion depth checking.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `err_msg!` or `err_ctx!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! use sutra::err_msg;
//! let err = err_msg!(Validation, "Invalid macro expansion");
//! assert!(matches!(err, sutra::SutraError::Validation { .. }));
//! ```
//!
//! All macro expansion errors (arity, recursion, substitution, etc.) use this system.
//!
//! ## Recursion and Expansion
//!
//! This module provides the core expansion engine, including recursion depth checks and trace recording. All errors related to recursion limits or invalid macro forms are reported using the canonical error system.

use std::collections::HashMap;

use crate::{
    ast::{AstNode, Expr, ParamList, Span, Spanned},
    err_src,
    macros::{
        check_arity, MacroDefinition, MacroExpansionContext, MacroExpansionResult,
        MacroExpansionStep, MacroOrigin, MacroTemplate, MAX_MACRO_RECURSION_DEPTH,
    },
    SutraError,
};

// =============================
// Type aliases to reduce verbosity
// =============================

/// Type alias for macro parameter bindings
type MacroBindings = HashMap<String, AstNode>;

/// Type alias for AST node slice
type AstNodeSlice = [AstNode];

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
    // Check recursion depth
    check_recursion_depth(depth, "macro", &call.span, env)?;

    match macro_def {
        MacroDefinition::Template(template) => {
            // Early validation: extract macro name from call
            let Expr::List(items, _) = &*call.value else {
                return Err(err_src!(
                    Eval,
                    "Macro call must be a list expression",
                    &env.source,
                    call.span
                ));
            };

            let first = items.first().ok_or_else(|| {
                err_src!(Eval, "Macro call cannot be empty", &env.source, call.span)
            })?;

            let Expr::Symbol(macro_name, _) = &*first.value else {
                return Err(err_src!(
                    Eval,
                    "Macro call head must be a symbol",
                    &env.source,
                    first.span
                ));
            };

            expand_template(template, call, macro_name, depth, env)
        }
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
    macro_name: &str,
    depth: usize,
    env: &MacroExpansionContext,
) -> MacroExpansionResult {
    check_recursion_depth(depth, macro_name, &call.span, env)?;

    // Early validation: extract arguments
    let (args, span) = match &*call.value {
        Expr::List(items, span) if !items.is_empty() => (&items[1..], span),
        _ => {
            return Err(err_src!(
                Internal,
                format!("Macro call to '{}' must be a non-empty list", macro_name),
                &env.source,
                call.span
            ));
        }
    };

    check_arity(args.len(), &template.params, macro_name, span)?;
    let bindings = bind_macro_params(args, &template.params, span);
    substitute_template(&template.body, &bindings, env, depth + 1, macro_name)
}

/// Binds macro parameters to arguments for template substitution.
/// Handles both regular and variadic parameters with proper list construction.
pub fn bind_macro_params(
    args: &AstNodeSlice,
    params: &ParamList,
    expr_span: &Span,
) -> MacroBindings {
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

    // Insert a special marker for variadic parameters to enable proper splicing
    bindings.insert(
        variadic_name.clone(),
        with_span(Expr::List(rest_args, *expr_span), expr_span),
    );

    // Also create a boolean marker for the variadic parameter name
    bindings.insert(
        format!("__variadic__{variadic_name}"),
        with_span(Expr::Bool(true, *expr_span), expr_span),
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
    macro_name: &str,
) -> MacroExpansionResult {
    // Step 1: Early validation - check recursion depth
    check_recursion_depth(depth, macro_name, &expr.span, env)?;

    let value = &*expr.value;

    // Step 2: Handle symbol substitution (early return)
    if let Expr::Symbol(name, _) = value {
        return substitute_symbol(name, bindings, expr);
    }

    // Step 3: Handle list processing with spreads (early return)
    if let Expr::List(items, _) = value {
        return substitute_list(items, bindings, env, depth, macro_name);
    }

    // Step 4: Handle other expression types (early returns)
    match value {
        Expr::Quote(inner, span) => substitute_quote(inner, span, expr),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => substitute_if_expression(
            condition,
            then_branch,
            else_branch,
            span,
            expr,
            bindings,
            env,
            depth,
            macro_name,
        ),
        _ => Ok(expr.clone()),
    }
}

/// Substitutes a symbol with its binding if available
fn substitute_symbol(name: &str, bindings: &MacroBindings, expr: &AstNode) -> MacroExpansionResult {
    Ok(bindings.get(name).cloned().unwrap_or_else(|| expr.clone()))
}

/// Substitutes elements in a list, handling spreads
fn substitute_list(
    items: &[AstNode],
    bindings: &MacroBindings,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> MacroExpansionResult {
    // Step 1: Process each item in the list
    let mut new_items = Vec::new();

    for item in items {
        let processed_items = substitute_list_item(item, bindings, env, depth, macro_name)?;
        new_items.extend(processed_items);
    }

    // Step 2: Create new list expression
    let span = items.first().unwrap().span;
    Ok(with_span(Expr::List(new_items, span), &span))
}

/// Substitutes a single list item, handling spreads
fn substitute_list_item(
    item: &AstNode,
    bindings: &MacroBindings,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> Result<Vec<AstNode>, SutraError> {
    // Step 1: Handle spread expressions (early return)
    if let Expr::Spread(inner) = &*item.value {
        let substituted = substitute_template(inner, bindings, env, depth + 1, macro_name)?;
        let Expr::List(elements, _) = &*substituted.value else {
            return Err(err_src!(
                Internal,
                "Spread expression must evaluate to a list",
                &env.source,
                inner.span
            ));
        };
        return Ok(elements.clone());
    }

    // Step 2: Handle regular expressions
    let substituted = substitute_template(item, bindings, env, depth + 1, macro_name)?;
    Ok(vec![substituted])
}

/// Substitutes quote expressions (preserves literal content)
fn substitute_quote(inner: &AstNode, span: &Span, expr: &AstNode) -> MacroExpansionResult {
    // Don't substitute inside quotes - they should be literal
    Ok(with_span(
        Expr::Quote(Box::new(inner.clone()), *span),
        &expr.span,
    ))
}

/// Substitutes if expressions recursively
fn substitute_if_expression(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    span: &Span,
    expr: &AstNode,
    bindings: &MacroBindings,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> MacroExpansionResult {
    // Step 1: Substitute condition
    let new_condition = substitute_template(condition, bindings, env, depth + 1, macro_name)?;

    // Step 2: Substitute then branch
    let new_then = substitute_template(then_branch, bindings, env, depth + 1, macro_name)?;

    // Step 3: Substitute else branch
    let new_else = substitute_template(else_branch, bindings, env, depth + 1, macro_name)?;

    // Step 4: Create new if expression
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

// =============================
// Macro expansion
// =============================

/// Recursively expands macros and records trace.
fn expand_macros_recursively_with_trace(
    node: AstNode,
    env: &mut MacroExpansionContext,
    depth: usize,
) -> MacroExpansionResult {
    if let Some((_macro_name, _provenance, expanded)) = expand_macro_once(&node, env, depth)? {
        // trace is already handled in expand_macro_def via env.trace
        return expand_macros_recursively_with_trace(expanded, env, depth + 1);
    }
    map_ast(node, &expand_macros_recursively_with_trace, env, depth)
}

/// Expands a macro call once, checking recursion depth.
fn expand_macro_once(
    node: &AstNode,
    env: &mut MacroExpansionContext,
    depth: usize,
) -> Result<Option<(String, MacroOrigin, AstNode)>, SutraError> {
    let Some(macro_name) = extract_macro_name_from_call(node) else {
        return Ok(None);
    };

    check_recursion_depth(depth, "macro", &node.span, env)?;

    // Clone MacroDefinition to avoid holding a borrow of env
    let (provenance, macro_def) = match env.lookup_macro(macro_name) {
        Some((prov, def)) => (prov, def.clone()),
        None => return Ok(None),
    };

    let expanded =
        expand_macro_definition(&macro_def, node, macro_name, depth, &mut *env, provenance)?;
    Ok(Some((macro_name.to_string(), provenance, expanded)))
}

/// Handles the actual expansion based on MacroDefinition type (Fn or Template).
fn expand_macro_definition(
    macro_def: &MacroDefinition,
    node: &AstNode,
    macro_name: &str,
    depth: usize,
    env: &mut MacroExpansionContext,
    provenance: MacroOrigin,
) -> MacroExpansionResult {
    let macro_name = macro_name.to_string();
    let original_node = node.clone();

    let result = match macro_def {
        MacroDefinition::Fn(func) => wrap_macro_error(func(node), &macro_name, &node.span, env),
        MacroDefinition::Template(template) => wrap_macro_error(
            expand_template(template, node, &macro_name, depth, env),
            &macro_name,
            &node.span,
            env,
        ),
    };

    if let Ok(expanded_node) = &result {
        // Record expansion trace
        env.trace.push(MacroExpansionStep {
            macro_name,
            provenance,
            input: original_node,
            output: expanded_node.clone(),
        });
    }

    result
}

/// Extracts the macro name from a macro call AST node.
fn extract_macro_name_from_call(node: &AstNode) -> Option<&str> {
    let Expr::List(items, _) = &*node.value else {
        return None;
    };
    let first = items.first()?;
    let Expr::Symbol(s, _) = &*first.value else {
        return None;
    };
    Some(s)
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
            let mut new_items = Vec::new();
            for item in items {
                new_items.push(f(item.clone(), env, depth + 1)?);
            }
            let list_expr = Expr::List(new_items, *span);
            let result = with_span(list_expr, &node.span);
            Ok(result)
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let cond = f(condition.as_ref().clone(), env, depth + 1)?;
            let then_b = f(then_branch.as_ref().clone(), env, depth + 1)?;
            let else_b = f(else_branch.as_ref().clone(), env, depth + 1)?;

            let if_expr = Expr::If {
                condition: Box::new(cond),
                then_branch: Box::new(then_b),
                else_branch: Box::new(else_b),
                span: *span,
            };
            let result = with_span(if_expr, &node.span);
            Ok(result)
        }
        Expr::Quote(inner, span) => {
            let new_inner = f(inner.as_ref().clone(), env, depth + 1)?;
            let quote_expr = Expr::Quote(Box::new(new_inner), *span);
            let result = with_span(quote_expr, &node.span);
            Ok(result)
        }
        Expr::Spread(inner) => {
            let new_inner = f(inner.as_ref().clone(), env, depth + 1)?;
            let spread_expr = Expr::Spread(Box::new(new_inner));
            let result = with_span(spread_expr, &node.span);
            Ok(result)
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
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(err_src!(
            Internal,
            format!("Macro recursion limit exceeded in '{}'", macro_name),
            &env.source,
            *span
        ));
    }
    Ok(())
}

/// Wraps macro errors with consistent formatting.
fn wrap_macro_error<T>(
    result: Result<T, SutraError>,
    macro_name: &str,
    span: &Span,
    env: &MacroExpansionContext,
) -> Result<T, SutraError> {
    result.map_err(|e| {
        err_src!(
            Internal,
            format!("Error in macro '{}': {}", macro_name, e),
            &env.source,
            *span
        )
    })
}

/// Creates a Spanned wrapper with consistent span handling.
fn with_span(value: Expr, original_span: &Span) -> AstNode {
    Spanned {
        value: value.into(),
        span: *original_span,
    }
}
