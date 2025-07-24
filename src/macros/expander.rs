//!
//! Manages template substitution, variadic forwarding, and recursive expansion,
//! ensuring proper error handling and recursion depth checking.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using miette-native error variants directly. See `src/errors.rs` for error types and usage rules.
//!
//! All macro expansion errors (arity, recursion, substitution, etc.) use this system.
//!
//! ## Recursion and Expansion
//!
//! This module provides the core expansion engine, including recursion depth checks and trace recording. All errors related to recursion limits or invalid macro forms are reported using the canonical error system.

use std::collections::HashMap;

use crate::prelude::*;
use crate::{
    ast::Span,
    errors,
    macros::{
        MacroDefinition, MacroExpansionContext, MacroExpansionResult,
        MacroExpansionStep, MacroTemplate, MAX_MACRO_RECURSION_DEPTH,
    },
    runtime::source::SourceContext,
    syntax::parser::to_source_span,
};

/// Expands all macro calls recursively within an AST node, tracing expansions.
/// This is the main entry point for macro expansion during evaluation.
pub fn expand_macros_recursively(
    mut node: AstNode,
    env: &mut MacroExpansionContext,
) -> MacroExpansionResult {
    let source_ctx = SourceContext::from_file(env.source.name(), env.source.inner());
    let mut depth = 0;
    // === Phase 1: Macro Expansion Loop ===
    loop {
        let macro_call = match extract_macro_call(&node) {
            Some((macro_name, _args)) => macro_name,
            None => break,
        };
        let (provenance, macro_def) = match env.lookup_macro(macro_call) {
            Some((prov, def)) => (prov, def.clone()),
            None => break,
        };
        if depth > MAX_MACRO_RECURSION_DEPTH {
            return Err(errors::runtime_general(
                format!("Macro recursion limit of {} exceeded (in '{}')", MAX_MACRO_RECURSION_DEPTH, macro_call),
                "macro recursion limit",
                &source_ctx,
                to_source_span(node.span),
            ).with_suggestion(format!("Check for infinite recursion in macro '{}'.", macro_call)));
        }
        let original_node = node.clone();
        let result = match &macro_def {
            MacroDefinition::Template(template) => expand_template(template, &node, env, &source_ctx, depth)?,
            MacroDefinition::Fn(func) => func(&node, &source_ctx)?,
        };
        env.trace.push(MacroExpansionStep {
            macro_name: macro_call.to_string(),
            provenance,
            input: original_node,
            output: result.clone(),
        });
        node = result;
        depth += 1;
    }
    // === Phase 2: Recursive Subform Expansion ===
    if let Expr::List(items, span) = &*node.value {
        let new_items = items.iter()
            .map(|item| expand_macros_recursively(item.clone(), env))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Spanned { value: Expr::List(new_items, *span).into(), span: node.span });
    }
    match &*node.value {
        Expr::If { condition, then_branch, else_branch, span } => {
            let new_condition = expand_macros_recursively(condition.as_ref().clone(), env)?;
            let new_then = expand_macros_recursively(then_branch.as_ref().clone(), env)?;
            let new_else = expand_macros_recursively(else_branch.as_ref().clone(), env)?;
            return Ok(Spanned {
                value: Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: Box::new(new_else),
                    span: *span,
                }.into(),
                span: node.span,
            });
        }
        Expr::Quote(inner, span) => {
            let new_inner = expand_macros_recursively(inner.as_ref().clone(), env)?;
            return Ok(Spanned { value: Expr::Quote(Box::new(new_inner), *span).into(), span: node.span });
        }
        Expr::Spread(inner) => {
            let new_inner = expand_macros_recursively(inner.as_ref().clone(), env)?;
            return Ok(Spanned { value: Expr::Spread(Box::new(new_inner)).into(), span: node.span });
        }
        _ => return Ok(node),
    }
}

/// Expands a macro template by substituting arguments into the template body.
/// Performs arity checks, parameter binding, and recursion depth validation.
fn expand_template(
    template: &MacroTemplate,
    call: &AstNode,
    env: &MacroExpansionContext,
    source_ctx: &SourceContext,
    depth: usize,
) -> MacroExpansionResult {
    // Extract macro name and arguments from a macro call
    let (macro_name, args, span) = match extract_macro_call_info(call) {
        Ok(t) => t,
        Err(e) => return Err(e),
    };
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(errors::runtime_general(
            format!("Macro recursion limit of {} exceeded", MAX_MACRO_RECURSION_DEPTH),
            "recursion limit",
            source_ctx,
            to_source_span(*span),
        ).with_suggestion(format!("Check for infinite recursion in macro '{}'", macro_name)));
    }
    // Arity check
    crate::macros::check_arity(args.len(), &template.params, macro_name, span, source_ctx)?;
    // Bind parameters (required and variadic)
    let mut bindings = HashMap::new();
    for (i, param_name) in template.params.required.iter().enumerate() {
        bindings.insert(param_name.clone(), args[i].clone());
    }
    if let Some(variadic_name) = &template.params.rest {
        let rest_args = if args.len() > template.params.required.len() {
            args[template.params.required.len()..].to_vec()
        } else {
            Vec::new()
        };
        // Patch: Always substitute (list ...rest_args) as the value for the variadic parameter
        let mut list_items = Vec::with_capacity(rest_args.len() + 1);
        // Add the 'list' symbol as the head
        list_items.push(Spanned {
            value: Expr::Symbol("list".to_string(), *span).into(),
            span: *span,
        });
        list_items.extend(rest_args);
        let list_node = Spanned {
            value: Expr::List(list_items, *span).into(),
            span: *span,
        };
        bindings.insert(variadic_name.clone(), list_node);
    }
    substitute_template(&template.body, &bindings, env, source_ctx, depth + 1)
}

/// Recursively substitutes macro parameters in template expressions.
fn substitute_template(
    expr: &AstNode,
    bindings: &HashMap<String, AstNode>,
    env: &MacroExpansionContext,
    source_ctx: &SourceContext,
    depth: usize,
) -> MacroExpansionResult {
    if depth > MAX_MACRO_RECURSION_DEPTH {
        return Err(errors::runtime_general(
            format!("Macro recursion limit of {} exceeded (in substitution)", MAX_MACRO_RECURSION_DEPTH),
            "macro recursion limit",
            source_ctx,
            to_source_span(expr.span),
        ));
    }
    // === Flat, Early-Return Structure ===
    if let Expr::Symbol(name, _) = &*expr.value {
        return Ok(bindings.get(name).cloned().unwrap_or_else(|| expr.clone()));
    }
    if let Expr::List(items, span) = &*expr.value {
        let mut new_items = Vec::with_capacity(items.len());
        for item in items {
            if let Expr::Spread(inner) = &*item.value {
                let substituted = substitute_template(inner, bindings, env, source_ctx, depth + 1)?;
                if let Expr::List(elements, _) = &*substituted.value {
                    new_items.extend(elements.clone());
                } else {
                    return Err(errors::type_mismatch(
                        "list",
                        substituted.type_name(),
                        source_ctx,
                        to_source_span(inner.span),
                    ).with_suggestion("Spread expressions (...) in macros must be bound to a list value."));
                }
                continue;
            }
            let substituted = substitute_template(item, bindings, env, source_ctx, depth + 1)?;
            new_items.push(substituted);
        }
        return Ok(Spanned { value: Expr::List(new_items, *span).into(), span: expr.span });
    }
    if let Expr::Quote(inner, span) = &*expr.value {
        return Ok(Spanned { value: Expr::Quote(inner.clone(), *span).into(), span: expr.span });
    }
    if let Expr::If { condition, then_branch, else_branch, span } = &*expr.value {
        let new_condition = substitute_template(condition, bindings, env, source_ctx, depth + 1)?;
        let new_then = substitute_template(then_branch, bindings, env, source_ctx, depth + 1)?;
        let new_else = substitute_template(else_branch, bindings, env, source_ctx, depth + 1)?;
        return Ok(Spanned {
            value: Expr::If {
                condition: Box::new(new_condition),
                then_branch: Box::new(new_then),
                else_branch: Box::new(new_else),
                span: *span,
            }.into(),
            span: expr.span,
        });
    }
    if let Expr::Spread(inner) = &*expr.value {
        let new_inner = substitute_template(inner, bindings, env, source_ctx, depth + 1)?;
        return Ok(Spanned { value: Expr::Spread(Box::new(new_inner)).into(), span: expr.span });
    }
    // All other cases: return as-is
    Ok(expr.clone())
}

/// Extracts macro name and arguments from a macro call, or returns None if not a macro call.
fn extract_macro_call(node: &AstNode) -> Option<(&str, &[AstNode])> {
    if let Expr::List(items, _) = &*node.value {
        if items.is_empty() {
            return None;
        }
        if let Expr::Symbol(macro_name, _) = &*items[0].value {
            return Some((macro_name.as_str(), &items[1..]));
        }
    }
    None
}

/// Extracts macro name and arguments from a macro call, or returns an error if invalid.
fn extract_macro_call_info(call: &AstNode) -> Result<(&str, &[AstNode], &Span), SutraError> {
    let Expr::List(items, span) = &*call.value else {
        let sc = SourceContext::from_file("macro-expander", format!("{:?}", call));
        return Err(errors::runtime_general(
            "Macro call must be a list expression",
            "macro call",
            &sc,
            to_source_span(call.span),
        ).with_suggestion("Macros must be called using list syntax, like `(my-macro ...)`."));
    };
    if items.is_empty() {
        let sc = SourceContext::from_file("macro-expander", format!("{:?}", call));
        return Err(errors::runtime_general(
            "Macro call cannot be empty",
            "macro call",
            &sc,
            to_source_span(*span),
        ));
    }
    let first = &items[0];
    let Expr::Symbol(macro_name, _) = &*first.value else {
        let sc = SourceContext::from_file("macro-expander", format!("{:?}", call));
        return Err(errors::runtime_general(
            "Macro call head must be a symbol",
            "macro call",
            &sc,
            to_source_span(first.span),
        ).with_suggestion("The first element of a macro call must be the macro's name."));
    };
    Ok((macro_name, &items[1..], span))
}
