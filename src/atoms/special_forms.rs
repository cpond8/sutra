//! Special forms for the Sutra language - Refactored
//!
//! This module provides core language constructs with simplified, unified logic.

use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    errors::{to_source_span, ErrorKind, ErrorReporting},
    runtime::{
        evaluate_ast_node, ConsCell, EvaluationContext, Lambda, NativeFn, SpannedResult,
        SpannedValue, Value,
    },
    syntax::{AstNode, Expr, ParamList, Span},
};

/// Creates a synthetic (do ...) expression from multiple body expressions
fn wrap_in_do(expressions: &[AstNode], span: Span) -> AstNode {
    if expressions.len() == 1 {
        return expressions[0].clone();
    }

    let do_symbol = AstNode {
        value: std::sync::Arc::new(Expr::Symbol("do".to_string(), span)),
        span,
    };

    let mut items = Vec::with_capacity(expressions.len() + 1);
    items.push(do_symbol);
    items.extend_from_slice(expressions);

    AstNode {
        value: std::sync::Arc::new(Expr::List(items, span)),
        span,
    }
}

/// Collects all symbol references in an AST node
fn collect_symbols(node: &AstNode) -> Vec<String> {
    let mut symbols = Vec::new();
    collect_symbols_helper(node, &mut symbols);
    symbols
}

fn collect_symbols_helper(node: &AstNode, symbols: &mut Vec<String>) {
    match &*node.value {
        Expr::Symbol(s, _) => symbols.push(s.clone()),
        Expr::List(items, _) => {
            // Skip quoted expressions
            if let Some(first) = items.first() {
                if matches!(&*first.value, Expr::Symbol(s, _) if s == "quote") {
                    return;
                }
            }
            for item in items {
                collect_symbols_helper(item, symbols);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_symbols_helper(condition, symbols);
            collect_symbols_helper(then_branch, symbols);
            collect_symbols_helper(else_branch, symbols);
        }
        _ => {}
    }
}

/// Captures free variables for closure creation
fn capture_environment(
    body: &AstNode,
    params: &ParamList,
    context: &EvaluationContext,
) -> HashMap<String, Value> {
    let all_symbols = collect_symbols(body);
    let mut captured = HashMap::new();

    for symbol in all_symbols {
        // Skip parameters
        if params.required.contains(&symbol) || params.rest.as_ref() == Some(&symbol) {
            continue;
        }

        if let Some(value) = context.get_var(&symbol) {
            captured.insert(symbol, value.clone());
        }
    }

    captured
}

/// Creates a lambda from parameters and body expressions
fn create_lambda(
    params: ParamList,
    body_exprs: &[AstNode],
    context: &EvaluationContext,
    span: Span,
) -> Value {
    let body = Box::new(wrap_in_do(body_exprs, span));
    let captured_env = capture_environment(&body, &params, context);

    Value::Lambda(Rc::new(Lambda {
        params,
        body,
        captured_env,
    }))
}

/// Builds a cons list from a slice of values
fn build_cons_list(values: &[Value]) -> Value {
    values.iter().rev().fold(Value::Nil, |acc, val| {
        Value::Cons(Rc::new(ConsCell {
            car: val.clone(),
            cdr: acc,
        }))
    })
}

pub const ATOM_DEFINE: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span)));
    }

    match &*args[0].value {
        // Variable definition: (define var value)
        Expr::Symbol(name, _) => {
            if args.len() != 2 {
                return Err(context.arity_mismatch(
                    "2 for variable definition",
                    args.len(),
                    to_source_span(*call_span),
                ));
            }
            let value = evaluate_ast_node(&args[1], context)?;
            context.set_var(name, value.value.clone());
            Ok(value)
        }

        // Function definition: (define (name params...) body...)
        Expr::List(items, _) => {
            if items.is_empty() {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "define".to_string(),
                        operand_type: "function name required".to_string(),
                    },
                    to_source_span(*call_span),
                ));
            }

            let name = match &*items[0].value {
                Expr::Symbol(s, _) => s.clone(),
                _ => {
                    return Err(context.report(
                        ErrorKind::TypeMismatch {
                            expected: "symbol".to_string(),
                            actual: "non-symbol".to_string(),
                        },
                        context.span_for_node(&items[0]),
                    ))
                }
            };

            let param_names: Result<Vec<String>, _> = items[1..]
                .iter()
                .map(|node| match &*node.value {
                    Expr::Symbol(s, _) => Ok(s.clone()),
                    _ => Err(context.report(
                        ErrorKind::TypeMismatch {
                            expected: "symbol".to_string(),
                            actual: "non-symbol".to_string(),
                        },
                        context.span_for_node(node),
                    )),
                })
                .collect();

            let params = ParamList {
                required: param_names?,
                rest: None,
                span: *call_span,
            };

            let lambda = create_lambda(params, &args[1..], context, *call_span);
            context.set_var(&name, lambda.clone());

            Ok(SpannedValue {
                value: lambda,
                span: *call_span,
            })
        }

        // Function definition with ParamList: (define ParamList{...} body...)
        Expr::ParamList(param_list) => {
            if param_list.required.is_empty() {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "define".to_string(),
                        operand_type: "function name required".to_string(),
                    },
                    to_source_span(param_list.span),
                ));
            }

            let name = param_list.required[0].clone();
            let params = ParamList {
                required: param_list.required[1..].to_vec(),
                rest: param_list.rest.clone(),
                span: param_list.span,
            };

            let lambda = create_lambda(params, &args[1..], context, *call_span);
            context.set_var(&name, lambda.clone());

            Ok(SpannedValue {
                value: lambda,
                span: *call_span,
            })
        }

        _ => Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "define".to_string(),
                operand_type: "invalid first argument".to_string(),
            },
            context.span_for_node(&args[0]),
        )),
    }
};

pub const ATOM_IF: NativeFn = |args, context, call_span| {
    if args.len() != 3 {
        return Err(context.arity_mismatch("3", args.len(), to_source_span(*call_span)));
    }

    let condition = evaluate_ast_node(&args[0], context)?;
    let branch = if condition.value.is_truthy() {
        &args[1]
    } else {
        &args[2]
    };
    evaluate_ast_node(branch, context)
};

pub const ATOM_COND: NativeFn = |args, context, call_span| {
    for clause_node in args {
        let clause = match &*clause_node.value {
            Expr::List(items, _) => items,
            _ => {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "cond".to_string(),
                        operand_type: "clause must be a list".to_string(),
                    },
                    context.span_for_node(clause_node),
                ))
            }
        };

        if clause.is_empty() {
            continue;
        }

        let is_else = matches!(&*clause[0].value, Expr::Symbol(s, _) if s == "else");

        if is_else {
            return if clause.len() > 1 {
                evaluate_ast_node(&clause[1], context)
            } else {
                Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "cond".to_string(),
                        operand_type: "else clause needs expression".to_string(),
                    },
                    context.span_for_node(clause_node),
                ))
            };
        }

        let condition = evaluate_ast_node(&clause[0], context)?;
        if condition.value.is_truthy() {
            return if clause.len() > 1 {
                evaluate_ast_node(&clause[1], context)
            } else {
                Ok(condition)
            };
        }
    }

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

pub const ATOM_AND: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Ok(SpannedValue {
            value: Value::Bool(true),
            span: *call_span,
        });
    }

    let mut result = SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    };
    for arg in args {
        result = evaluate_ast_node(arg, context)?;
        if !result.value.is_truthy() {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: result.span,
            });
        }
    }
    Ok(result)
};

pub const ATOM_OR: NativeFn = |args, context, call_span| {
    let mut result = SpannedValue {
        value: Value::Bool(false),
        span: *call_span,
    };
    for arg in args {
        result = evaluate_ast_node(arg, context)?;
        if result.value.is_truthy() {
            return Ok(result);
        }
    }
    Ok(result)
};

pub const ATOM_LAMBDA: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span)));
    }

    let params = match &*args[0].value {
        Expr::ParamList(pl) => pl.clone(),
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "lambda".to_string(),
                    operand_type: "first argument must be parameter list".to_string(),
                },
                to_source_span(*call_span),
            ))
        }
    };

    let lambda = create_lambda(params, &args[1..], context, *call_span);
    Ok(SpannedValue {
        value: lambda,
        span: *call_span,
    })
};

pub const ATOM_LET: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span)));
    }

    let bindings = match &*args[0].value {
        Expr::List(pairs, _) => pairs,
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "let".to_string(),
                    operand_type: "first argument must be binding list".to_string(),
                },
                to_source_span(args[0].span),
            ))
        }
    };

    let mut new_context = context.with_new_frame();

    for pair in bindings {
        let (name, value_expr) = match &*pair.value {
            Expr::List(items, _) if items.len() == 2 => match &*items[0].value {
                Expr::Symbol(name, _) => (name.clone(), &items[1]),
                _ => {
                    return Err(context.report(
                        ErrorKind::TypeMismatch {
                            expected: "symbol".to_string(),
                            actual: "non-symbol".to_string(),
                        },
                        to_source_span(pair.span),
                    ))
                }
            },
            _ => {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "let".to_string(),
                        operand_type: "binding must be (name value) pair".to_string(),
                    },
                    to_source_span(pair.span),
                ))
            }
        };

        let value = evaluate_ast_node(value_expr, &mut new_context)?;
        new_context.set_var(&name, value.value);
    }

    let body = wrap_in_do(&args[1..], *call_span);
    evaluate_ast_node(&body, &mut new_context)
};

pub fn call_lambda(
    lambda: &Lambda,
    args: &[Value],
    context: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    let mut new_context = context.with_new_frame();

    // Restore captured environment
    for (name, value) in &lambda.captured_env {
        new_context.set_var(name, value.clone());
    }

    // Validate and bind parameters
    let required_count = lambda.params.required.len();
    let is_variadic = lambda.params.rest.is_some();

    if (!is_variadic && args.len() != required_count)
        || (is_variadic && args.len() < required_count)
    {
        let expected = if is_variadic {
            format!("{}+", required_count)
        } else {
            required_count.to_string()
        };
        return Err(context.arity_mismatch(&expected, args.len(), to_source_span(*call_span)));
    }

    // Bind required parameters
    for (name, value) in lambda.params.required.iter().zip(args.iter()) {
        new_context.set_var(name, value.clone());
    }

    // Bind variadic parameter if present
    if let Some(rest_name) = &lambda.params.rest {
        let rest_args = &args[required_count..];
        let rest_list = build_cons_list(rest_args);
        new_context.set_var(rest_name, rest_list);
    }

    evaluate_ast_node(&lambda.body, &mut new_context)
}
