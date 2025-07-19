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

/// Type alias for common macro substitution parameters
type MacroSubstitutionParams<'a> = (&'a MacroBindings, &'a MacroExpansionContext, usize, &'a str);

// =============================
// Unified context for macro operations
// =============================

/// Unified context for macro substitution and expansion operations
struct MacroContext<'a> {
    bindings: &'a MacroBindings,
    env: &'a MacroExpansionContext,
    depth: usize,
    macro_name: &'a str,
}

impl<'a> MacroContext<'a> {
    fn new(
        bindings: &'a MacroBindings,
        env: &'a MacroExpansionContext,
        depth: usize,
        macro_name: &'a str,
    ) -> Self {
        Self {
            bindings,
            env,
            depth,
            macro_name,
        }
    }

    fn with_depth(&self, depth: usize) -> Self {
        Self {
            bindings: self.bindings,
            env: self.env,
            depth,
            macro_name: self.macro_name,
        }
    }
}

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
            // Extract macro name from call node with guard clauses
            let items = match &*call.value {
                Expr::List(items, _) => items,
                _ => {
                    return Err(err_src!(
                        Eval,
                        "Macro call must be a list expression",
                        &env.source,
                        call.span
                    ));
                }
            };

            let first = items.first().ok_or_else(|| {
                err_src!(Eval, "Macro call cannot be empty", &env.source, call.span)
            })?;

            let macro_name = match &*first.value {
                Expr::Symbol(s, _) => s,
                _ => {
                    return Err(err_src!(
                        Eval,
                        "Macro call head must be a symbol",
                        &env.source,
                        first.span
                    ));
                }
            };

            expand_template(template, call, macro_name, depth, env)
        }
        MacroDefinition::Fn(func) => func(call),
    }
}

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
    let ctx = MacroContext::new(bindings, env, depth, macro_name);
    substitute_template_with_context(expr, &ctx)
}

/// Template substitution implementation using unified context.
fn substitute_template_with_context(expr: &AstNode, ctx: &MacroContext) -> MacroExpansionResult {
    check_recursion_depth(ctx.depth, ctx.macro_name, &expr.span, ctx.env)?;
    // Symbol substitution (including variadic)
    if let Expr::Symbol(name, _) = &*expr.value {
        if let Some(result) = substitute_symbol(name, expr, ctx.bindings) {
            return result;
        }
    }
    // Quoted expressions: do not descend into them
    if let Expr::Quote(inner, span) = &*expr.value {
        return Ok(with_span(Expr::Quote(inner.clone(), *span), &expr.span));
    }
    // List: recurse
    if let Expr::List(items, _) = &*expr.value {
        let list_ctx = ctx.with_depth(ctx.depth + 1);
        return substitute_list(&list_ctx, items);
    }
    // If: recurse
    if let Expr::If {
        condition,
        then_branch,
        else_branch,
        span,
    } = &*expr.value
    {
        let ctx = ctx.with_depth(ctx.depth + 1);
        return substitute_if(condition, then_branch, else_branch, span, &expr.span, &ctx);
    }
    // Default: return as is (atomic types)
    Ok(expr.clone())
}

// =============================
// Internal expansion helpers
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

    let expanded = expand_macro_definition(&macro_def, node, macro_name, depth, env, provenance)?;
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
        record_macro_expansion(
            &mut env.trace,
            macro_name,
            provenance,
            original_node,
            expanded_node.clone(),
        );
    }

    result
}

/// Records a single macro expansion step in the trace.
fn record_macro_expansion(
    trace: &mut Vec<MacroExpansionStep>,
    macro_name: String,
    provenance: MacroOrigin,
    input: AstNode,
    output: AstNode,
) {
    trace.push(MacroExpansionStep {
        macro_name,
        provenance,
        input,
        output,
    });
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
// Template substitution helpers
// =============================

/// Handles symbol substitution during template expansion, including variadic splicing.
fn substitute_symbol(
    name: &str,
    expr: &AstNode,
    bindings: &MacroBindings,
) -> Option<MacroExpansionResult> {
    let bound_node = bindings.get(name)?;

    // If it's a variadic parameter, convert its list to a `(list ...)` call
    if bindings.contains_key(&format!("__variadic__{name}")) {
        if let Expr::List(elements, _) = &*bound_node.value {
            return Some(Ok(make_list_call(elements, &expr.span)));
        }
    }

    Some(Ok(bound_node.clone()))
}

/// Creates a `(list ...)` AST node from a vector of elements.
fn make_list_call(elements: &AstNodeSlice, span: &Span) -> AstNode {
    let mut list_call = Vec::with_capacity(elements.len() + 1);
    list_call.push(with_span(Expr::Symbol("list".to_string(), *span), span));
    list_call.extend_from_slice(elements);
    with_span(Expr::List(list_call, *span), span)
}

/// List substitution implementation using unified context.
fn substitute_list(ctx: &MacroContext, items: &AstNodeSlice) -> MacroExpansionResult {
    let mut new_items = Vec::new();

    for item in items {
        // Handle explicit spread (e.g., ...args)
        if try_handle_spread_item(
            item,
            ctx.bindings,
            &mut new_items,
            ctx.env,
            ctx.depth,
            ctx.macro_name,
        )? {
            continue;
        }
        // Handle regular substitution (no implicit splicing)
        handle_regular_item(
            item,
            ctx.bindings,
            &mut new_items,
            ctx.env,
            ctx.depth,
            ctx.macro_name,
        )?;
    }

    let span = items.first().unwrap().span;
    Ok(with_span(Expr::List(new_items, span), &span))
}

/// Attempts to handle a spread item (`...expr`). Returns true if handled.
fn try_handle_spread_item(
    item: &AstNode,
    bindings: &MacroBindings,
    new_items: &mut Vec<AstNode>,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> Result<bool, SutraError> {
    if let Expr::Spread(inner) = &*item.value {
        substitute_spread_item_internal(inner, bindings, new_items, env, depth, macro_name)?;
        return Ok(true);
    }
    Ok(false)
}

/// Handles regular (non-spread) items, including direct variadic param reference.
fn handle_regular_item(
    item: &AstNode,
    bindings: &MacroBindings,
    new_items: &mut Vec<AstNode>,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> Result<(), SutraError> {
    // Check if this is a variadic parameter symbol that should be inserted as a list
    if let Some(name) = extract_variadic_param_name(item, bindings) {
        let bound_node = &bindings[name];
        new_items.push(bound_node.clone());
        return Ok(());
    }

    // Otherwise, substitute normally and push the result
    let substituted_item = substitute_and_collect(item, (bindings, env, depth, macro_name))?;
    new_items.push(substituted_item);
    Ok(())
}

/// Internal spread item handling with variadic parameter support.
fn substitute_spread_item_internal(
    inner: &AstNode,
    bindings: &MacroBindings,
    new_items: &mut Vec<AstNode>,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> Result<(), SutraError> {
    // If it's a variadic parameter symbol, splice its elements directly
    if let Some(name) = extract_variadic_param_name(inner, bindings) {
        let bound_node = &bindings[name];
        let Expr::List(elements, _) = &*bound_node.value else {
            return Err(err_src!(
                Internal,
                format!(
                    "Variadic parameter '{}' is not a list in '{}'",
                    name, macro_name
                ),
                &env.source,
                inner.span
            ));
        };

        extend_with_elements(new_items, elements);
        return Ok(());
    }

    // Otherwise, substitute the inner expression and require it to be a list for splicing
    handle_non_symbol_spread_internal(inner, bindings, new_items, env, depth, macro_name)
}

/// Internal non-symbol spread handling with list validation.
fn handle_non_symbol_spread_internal(
    inner: &AstNode,
    bindings: &MacroBindings,
    new_items: &mut Vec<AstNode>,
    env: &MacroExpansionContext,
    depth: usize,
    macro_name: &str,
) -> Result<(), SutraError> {
    let substituted = substitute_template(inner, bindings, env, depth + 1, macro_name)?;
    if let Expr::List(elements, _) = &*substituted.value {
        extend_with_elements(new_items, elements);
    } else {
        return Err(err_src!(
            Internal,
            format!(
                "Spread argument did not evaluate to a list in '{}'",
                macro_name
            ),
            &env.source,
            inner.span
        ));
    }
    Ok(())
}

/// Substitutes parameters within an `If` expression's branches.
fn substitute_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    if_span: &Span,
    original_span: &Span,
    ctx: &MacroContext,
) -> MacroExpansionResult {
    let new_condition = substitute_template(
        condition,
        ctx.bindings,
        ctx.env,
        ctx.depth + 1,
        ctx.macro_name,
    )?;
    let new_then = substitute_template(
        then_branch,
        ctx.bindings,
        ctx.env,
        ctx.depth + 1,
        ctx.macro_name,
    )?;
    let new_else = substitute_template(
        else_branch,
        ctx.bindings,
        ctx.env,
        ctx.depth + 1,
        ctx.macro_name,
    )?;

    Ok(with_span(
        Expr::If {
            condition: Box::new(new_condition),
            then_branch: Box::new(new_then),
            else_branch: Box::new(new_else),
            span: *if_span,
        },
        original_span,
    ))
}

/// Checks if a node is a variadic parameter symbol and returns its name.
fn extract_variadic_param_name<'a>(node: &'a AstNode, bindings: &MacroBindings) -> Option<&'a str> {
    let Expr::Symbol(name, _) = &*node.value else {
        return None;
    };

    if !is_variadic_param(name, bindings) {
        return None;
    }

    Some(name)
}

/// Checks if a given symbol name corresponds to a variadic parameter in the bindings.
fn is_variadic_param(name: &str, bindings: &MacroBindings) -> bool {
    bindings.contains_key(&format!("__variadic__{name}"))
}

// =============================
// AST traversal helpers
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
            let new_items: Result<Vec<_>, _> = items
                .iter()
                .map(|item| f(item.clone(), env, depth + 1))
                .collect();
            Ok(with_span(Expr::List(new_items?, *span), &node.span))
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
            Ok(with_span(
                Expr::If {
                    condition: Box::new(cond),
                    then_branch: Box::new(then_b),
                    else_branch: Box::new(else_b),
                    span: *span,
                },
                &node.span,
            ))
        }
        Expr::Quote(inner, span) => {
            let new_inner = f((**inner).clone(), env, depth + 1)?;
            Ok(with_span(
                Expr::Quote(Box::new(new_inner), *span),
                &node.span,
            ))
        }
        Expr::Spread(inner) => {
            let new_inner = f((**inner).clone(), env, depth + 1)?;
            Ok(with_span(Expr::Spread(Box::new(new_inner)), &node.span))
        }
        // Atomic types (Symbol, Path, String, Number, Bool) and ParamList don't need traversal
        _ => Ok(node),
    }
}

// =============================
// Helper functions for common operations
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

/// Extends a vector with cloned elements from a slice.
fn extend_with_elements(new_items: &mut Vec<AstNode>, elements: &[AstNode]) {
    for element in elements {
        new_items.push(element.clone());
    }
}

/// Substitutes an item and returns the result.
fn substitute_and_collect(
    item: &AstNode,
    params: MacroSubstitutionParams,
) -> Result<AstNode, SutraError> {
    let (bindings, env, depth, macro_name) = params;
    substitute_template(item, bindings, env, depth + 1, macro_name)
}

// =============================
// Utility functions
// =============================

// Creates a Spanned wrapper with consistent span handling.
fn with_span(value: Expr, original_span: &Span) -> AstNode {
    Spanned {
        value: value.into(),
        span: *original_span,
    }
}
