use std::rc::Rc;

use crate::prelude::*;
use crate::{
    ast::value::Lambda,
    atoms::{
        helpers::{validate_special_form_arity, validate_special_form_min_arity},
        SpecialFormAtomFn,
    },
    runtime::eval,
    syntax::parser::to_source_span,
};
use miette::NamedSource;

/// Implements the (if ...) special form with lazy evaluation.
pub const ATOM_IF: SpecialFormAtomFn = |args, context, _span| {
    validate_special_form_arity(args, 3, "if")?;
    let condition = &args[0];
    let then_branch = &args[1];
    let else_branch = &args[2];

    let (is_true, next_world) = eval::evaluate_condition_as_bool(condition, context)?;
    let mut sub_context = context.clone_with_new_lexical_frame();
    sub_context.world = &next_world;

    let branch = if is_true { then_branch } else { else_branch };
    eval::evaluate_ast_node(branch, &mut sub_context)
};

/// Implements the (lambda ...) special form.
pub const ATOM_LAMBDA: SpecialFormAtomFn = |args, context, span| {
    validate_special_form_min_arity(args, 2, "lambda")?;
    // Parse parameter list
    let param_list = match &*args[0].value {
        Expr::ParamList(pl) => pl.clone(),
        _ => {
            return Err(SutraError::RuntimeGeneral {
                message: "lambda: first argument must be a parameter list".to_string(),
                src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                span: to_source_span(*span),
                suggestion: None,
            });
        }
    };

    // Validate parameter names (no duplicates, all symbols)
    let mut seen = std::collections::HashSet::new();
    for name in &param_list.required {
        if !seen.insert(name) {
            return Err(SutraError::RuntimeGeneral {
                message: format!("lambda: duplicate parameter '{}'", name),
                src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                span: to_source_span(*span),
                suggestion: None,
            });
        }
    }
    if let Some(rest) = &param_list.rest {
        if !seen.insert(rest) {
            return Err(SutraError::RuntimeGeneral {
                message: format!("lambda: duplicate variadic parameter '{}'", rest),
                src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                span: to_source_span(*span),
                suggestion: None,
            });
        }
    }

    // Body: single or multiple expressions
    let body = if args.len() == 2 {
        Box::new(args[1].clone())
    } else {
        let mut exprs = Vec::with_capacity(args.len() - 1);
        for expr in args.iter().skip(1) {
            exprs.push(expr.clone());
        }
        let do_expr = Expr::List(
            std::iter::once(AstNode {
                value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *span)),
                span: *span,
            })
            .chain(exprs)
            .collect(),
            *span,
        );
        Box::new(AstNode {
            value: std::sync::Arc::new(do_expr),
            span: *span,
        })
    };

    // Capture the current lexical environment by flattening all frames
    let mut captured_env = std::collections::HashMap::new();
    for frame in &context.lexical_env {
        for (key, value) in frame {
            captured_env.insert(key.clone(), value.clone());
        }
    }
    Ok((
        Value::Lambda(Rc::new(Lambda {
            params: param_list,
            body,
            captured_env,
        })),
        context.world.clone(),
    ))
};

/// Implements the (let ...) special form.
pub const ATOM_LET: SpecialFormAtomFn = |args, context, span| {
    validate_special_form_min_arity(args, 2, "let")?;
    // Parse bindings
    let bindings = match &*args[0].value {
        Expr::List(pairs, _) => pairs,
        _ => {
            return Err(SutraError::RuntimeGeneral {
                message: "let: first argument must be a list of bindings".to_string(),
                src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                span: to_source_span(*span),
                suggestion: None,
            });
        }
    };

    let mut new_context = context.clone_with_new_lexical_frame();

    // Evaluate and bind each (name value) pair in order
    for pair in bindings {
        let (name, value_expr) = match &*pair.value {
            Expr::List(items, _) if items.len() == 2 => match &*items[0].value {
                Expr::Symbol(name, _) => (name.clone(), &items[1]),
                _ => {
                    return Err(SutraError::RuntimeGeneral {
                        message: "let: binding name must be a symbol".to_string(),
                        src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                        span: to_source_span(*span),
                        suggestion: None,
                    })
                }
            },
            _ => {
                return Err(SutraError::RuntimeGeneral {
                    message: "let: each binding must be a (name value) pair".to_string(),
                    src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
                    span: to_source_span(*span),
                    suggestion: None,
                });
            }
        };
        let (value, _) = eval::evaluate_ast_node(value_expr, &mut new_context)?;
        new_context.set_lexical_var(&name, value);
    }

    // Body: single or multiple expressions
    let body = if args.len() == 2 {
        &args[1]
    } else {
        // Wrap in (do ...)
        let mut exprs = Vec::with_capacity(args.len() - 1);
        for expr in args.iter().skip(1) {
            exprs.push(expr.clone());
        }
        let do_expr = Expr::List(
            std::iter::once(AstNode {
                value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *span)),
                span: *span,
            })
            .chain(exprs)
            .collect(),
            *span,
        );
        &AstNode {
            value: std::sync::Arc::new(do_expr),
            span: *span,
        }
    };

    let (result, world) = eval::evaluate_ast_node(body, &mut new_context)?;
    Ok((result, world))
};

/// Applies a Lambda value to arguments in the given evaluation context.
pub fn call_lambda(
    lambda: &Lambda,
    args: &[Value],
    context: &mut eval::EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let mut new_context = context.clone_with_new_lexical_frame();

    // Restore the captured lexical environment
    for (key, value) in &lambda.captured_env {
        new_context.set_lexical_var(key, value.clone());
    }

    // Bind parameters
    let fixed = lambda.params.required.len();
    let variadic = lambda.params.rest.is_some();
    if (!variadic && args.len() != fixed) || (variadic && args.len() < fixed) {
        let msg = format!(
            "lambda: expected {}{} arguments, got {}",
            fixed,
            if variadic { "+" } else { "" },
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/special_forms.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
    }
    for (name, value) in lambda.params.required.iter().zip(args.iter()) {
        new_context.set_lexical_var(name, value.clone());
    }
    if let Some(rest) = &lambda.params.rest {
        let rest_args = args[fixed..].to_vec();
        new_context.set_lexical_var(rest, Value::List(rest_args));
    }

    // Evaluate body in new context
    let (result, world) = eval::evaluate_ast_node(&lambda.body, &mut new_context)?;
    Ok((result, world))
}
