use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::{
    ast::{
        spanned_value::{SpannedResult, SpannedValue},
        value::{ConsCell, Lambda, NativeFn},
        AstNode, Expr, ParamList, Span, Value,
    },
    engine::{evaluate_ast_node, EvaluationContext},
    errors::{to_source_span, ErrorKind, ErrorReporting},
    prelude::*,
};

fn find_and_capture_free_variables(
    body: &AstNode,
    params: &ParamList,
    context: &EvaluationContext,
) -> HashMap<String, Value> {
    fn collect_symbols(node: &AstNode, symbols: &mut HashSet<String>) {
        match &*node.value {
            Expr::Symbol(s, _) => {
                symbols.insert(s.clone());
            }
            Expr::List(items, _) => {
                if items
                    .first()
                    .is_some_and(|head| matches!(&*head.value, Expr::Symbol(s, _) if s == "quote"))
                {
                    // Do not descend into quoted expressions
                    return;
                }
                for item in items {
                    collect_symbols(item, symbols);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                collect_symbols(condition, symbols);
                collect_symbols(then_branch, symbols);
                collect_symbols(else_branch, symbols);
            }
            _ => {}
        }
    }

    let mut symbols = HashSet::new();
    collect_symbols(body, &mut symbols);

    // Remove parameters from the set of symbols; the rest are free variables.
    for p in &params.required {
        symbols.remove(p);
    }
    if let Some(rest) = &params.rest {
        symbols.remove(rest);
    }

    // Capture the values of the free variables from the current environment.
    let mut captured_env = HashMap::new();
    for symbol in symbols {
        if let Some(value) = context.get_var(&symbol) {
            captured_env.insert(symbol, value.clone());
        }
    }

    captured_env
}

/// Implements the (define ...) special form for global bindings.
pub const ATOM_DEFINE: NativeFn = |args, context, call_span| {
    // 1. Validate arity: (define <name> <value>)
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let name_expr = &args[0];
    let value_expr = &args[1];

    // 2. Handle variable definition: (define my-var 100)
    if let Expr::Symbol(name, _) = &*name_expr.value {
        // Evaluate the value expression in the current context.
        let spanned_value = evaluate_ast_node(value_expr, context)?;

        // Patch: Set in lexical frame if in local scope, else global world state
        if context.env.len() > 1 {
            context.set_var(name, spanned_value.value.clone());
        } else {
            let path = Path(vec![name.clone()]);
            context
                .world
                .borrow_mut()
                .set(&path, spanned_value.value.clone());
        }

        return Ok(spanned_value);
    }

    // 3. Handle function definition: (define (my-func x) (+ x 1))
    if let Expr::List(items, list_span) = &*name_expr.value {
        return handle_function_definition_list(items, list_span, value_expr, context, call_span);
    }

    // 4. Handle function definition with ParamList: (define (ParamList { required: ["my-func", "x"], ... }) (+ x 1))
    if let Expr::ParamList(param_list) = &*name_expr.value {
        return handle_function_definition_paramlist(param_list, value_expr, context, call_span);
    }

    // 5. If the first argument is not a symbol, list, or param list, it's an error.
    Err(context.report(
        ErrorKind::InvalidOperation {
            operation: "define".to_string(),
            operand_type: "invalid argument type".to_string(),
        },
        context.span_for_node(name_expr),
    ))
};

/// Handle function definition when the first argument is a List (legacy format)
fn handle_function_definition_list(
    items: &[AstNode],
    list_span: &Span,
    value_expr: &AstNode,
    context: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    if items.is_empty() {
        return Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "define".to_string(),
                operand_type: "function definition requires a name".to_string(),
            },
            to_source_span(*call_span),
        ));
    }

    // The first item is the function name.
    let function_name = match &*items[0].value {
        Expr::Symbol(s, _) => s.clone(),
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "define".to_string(),
                    operand_type: "function name must be a symbol".to_string(),
                },
                context.span_for_node(&items[0]),
            ));
        }
    };

    // The rest of the items are the parameters.
    let params_nodes = &items[1..];
    let mut required = vec![];
    let mut rest = None;

    for param_node in params_nodes {
        match &*param_node.value {
            Expr::Symbol(s, _) => required.push(s.clone()),
            Expr::Spread(spread_expr) => {
                if let Expr::Symbol(s, _) = &*spread_expr.value {
                    rest = Some(s.clone());
                    break; // No params after a variadic parameter.
                } else {
                    return Err(context.report(
                        ErrorKind::TypeMismatch {
                            expected: "symbol".to_string(),
                            actual: "non-symbol".to_string(),
                        },
                        context.span_for_node(spread_expr),
                    ));
                }
            }
            _ => {
                return Err(context.report(
                    ErrorKind::TypeMismatch {
                        expected: "symbol".to_string(),
                        actual: "non-symbol".to_string(),
                    },
                    context.span_for_node(param_node),
                ));
            }
        }
    }

    let params = crate::ast::ParamList {
        required,
        rest,
        span: *list_span,
    };

    create_and_store_lambda(function_name, params, value_expr, context, call_span)
}

/// Handle function definition when the first argument is a ParamList (current parser format)
fn handle_function_definition_paramlist(
    param_list: &crate::ast::ParamList,
    value_expr: &AstNode,
    context: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    if param_list.required.is_empty() {
        return Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "define".to_string(),
                operand_type: "function definition requires a name".to_string(),
            },
            to_source_span(param_list.span),
        ));
    }

    // The first parameter is the function name
    let function_name = param_list.required[0].clone();

    // The rest are the actual parameters
    let function_params = crate::ast::ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span,
    };

    create_and_store_lambda(function_name, function_params, value_expr, context, call_span)
}

/// Create a lambda and store it in the appropriate scope
fn create_and_store_lambda(
    function_name: String,
    params: crate::ast::ParamList,
    value_expr: &AstNode,
    context: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    let body = Box::new(value_expr.clone());
    let captured_env = find_and_capture_free_variables(&body, &params, context);

    let lambda = Value::Lambda(Rc::new(Lambda {
        params,
        body,
        captured_env,
    }));

    // Patch: Set in lexical frame if in local scope, else global world state
    if context.env.len() > 1 {
        context.set_var(&function_name, lambda.clone());
    } else {
        let path = Path(vec![function_name.clone()]);
        context.world.borrow_mut().set(&path, lambda.clone());
    }

    Ok(SpannedValue {
        value: lambda,
        span: *call_span,
    })
}

/// Implements the (if ...) special form with lazy evaluation.
pub const ATOM_IF: NativeFn = |args, context, call_span| {
    if args.len() != 3 {
        return Err(context.arity_mismatch("3", args.len(), to_source_span(*call_span)));
    }
    let condition = &args[0];
    let then_branch = &args[1];
    let else_branch = &args[2];

    let condition_result = evaluate_ast_node(condition, context)?;
    let branch = if condition_result.value.is_truthy() {
        then_branch
    } else {
        else_branch
    };
    evaluate_ast_node(branch, context)
};

/// Implements the (cond ...) special form with lazy evaluation.
pub const ATOM_COND: NativeFn = |args, context, call_span| {
    for clause_node in args {
        let clause = match &*clause_node.value {
            Expr::List(items, _) => items,
            _ => {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "cond".to_string(),
                        operand_type: "each clause must be a list".to_string(),
                    },
                    context.span_for_node(clause_node),
                ));
            }
        };

        if clause.is_empty() {
            continue; // Skip empty clauses
        }

        let condition = &clause[0];
        let is_else = match &*condition.value {
            Expr::Symbol(s, _) => s == "else",
            _ => false,
        };

        if is_else {
            if clause.len() > 1 {
                return evaluate_ast_node(&clause[1], context);
            } else {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "cond".to_string(),
                        operand_type: "'else' clause must have an expression".to_string(),
                    },
                    context.span_for_node(clause_node),
                ));
            }
        }

        let condition_result = evaluate_ast_node(condition, context)?;
        if condition_result.value.is_truthy() {
            if clause.len() > 1 {
                return evaluate_ast_node(&clause[1], context);
            } else {
                // If condition is true and there's no expression, return the condition result
                return Ok(condition_result);
            }
        }
    }

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    }) // No condition was met
};

/// Implements the (and ...) special form with short-circuiting.
pub const ATOM_AND: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Ok(SpannedValue {
            value: Value::Bool(true),
            span: *call_span,
        });
    }

    let mut last_result = SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    };

    for arg in args {
        let result = evaluate_ast_node(arg, context)?;
        if !result.value.is_truthy() {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: result.span,
            });
        }
        last_result = result;
    }
    Ok(last_result)
};

/// Implements the (or ...) special form with short-circuiting.
pub const ATOM_OR: NativeFn = |args, context, call_span| {
    let mut last_result = SpannedValue {
        value: Value::Bool(false),
        span: *call_span,
    };
    for arg in args {
        last_result = evaluate_ast_node(arg, context)?;
        if last_result.value.is_truthy() {
            return Ok(last_result);
        }
    }
    Ok(last_result)
};

/// Implements the (lambda ...) special form.
pub const ATOM_LAMBDA: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(
            context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span))
        );
    }
    // Parse parameter list
    let param_list = match &*args[0].value {
        Expr::ParamList(pl) => pl.clone(),
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "lambda".to_string(),
                    operand_type: "first argument must be a parameter list".to_string(),
                },
                to_source_span(*call_span),
            ));
        }
    };

    // Validate parameter names (no duplicates, all symbols)
    let mut seen = std::collections::HashSet::new();
    for name in &param_list.required {
        if !seen.insert(name) {
            return Err(context.report(
                ErrorKind::DuplicateDefinition {
                    symbol: name.clone(),
                    original_location: to_source_span(*call_span),
                },
                to_source_span(*call_span),
            ));
        }
    }
    if let Some(rest) = &param_list.rest {
        if !seen.insert(rest) {
            return Err(context.report(
                ErrorKind::DuplicateDefinition {
                    symbol: rest.clone(),
                    original_location: to_source_span(*call_span),
                },
                to_source_span(*call_span),
            ));
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
                value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *call_span)),
                span: *call_span,
            })
            .chain(exprs)
            .collect(),
            *call_span,
        );
        Box::new(AstNode {
            value: std::sync::Arc::new(do_expr),
            span: *call_span,
        })
    };

    let captured_env = find_and_capture_free_variables(&body, &param_list, context);

    Ok(SpannedValue {
        value: Value::Lambda(Rc::new(Lambda {
            params: param_list,
            body,
            captured_env,
        })),
        span: *call_span,
    })
};

/// Implements the (let ...) special form.
pub const ATOM_LET: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(
            context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span))
        );
    }
    // Parse bindings
    let bindings = match &*args[0].value {
        Expr::List(pairs, _) => pairs,
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "let".to_string(),
                    operand_type: "first argument must be a list of bindings".to_string(),
                },
                to_source_span(args[0].span),
            ));
        }
    };

    let mut new_context = context.with_new_frame();

    // Evaluate and bind each (name value) pair in order
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
                    ));
                }
            },
            _ => {
                return Err(context.report(
                    ErrorKind::InvalidOperation {
                        operation: "let".to_string(),
                        operand_type: "each binding must be a (name value) pair".to_string(),
                    },
                    to_source_span(pair.span),
                ));
            }
        };
        let spanned_value = evaluate_ast_node(value_expr, &mut new_context)?;
        new_context.set_var(&name, spanned_value.value);
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
                value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *call_span)),
                span: *call_span,
            })
            .chain(exprs)
            .collect(),
            *call_span,
        );
        &AstNode {
            value: std::sync::Arc::new(do_expr),
            span: *call_span,
        }
    };

    evaluate_ast_node(body, &mut new_context)
};

/// Applies a Lambda value to arguments in the given evaluation context.
pub fn call_lambda(
    lambda: &Lambda,
    args: &[Value],
    context: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    let mut new_context = context.with_new_frame();

    // Bind parameters in the new top frame
    let fixed = lambda.params.required.len();
    let variadic = lambda.params.rest.is_some();
    if (!variadic && args.len() != fixed) || (variadic && args.len() < fixed) {
        return Err(context.arity_mismatch(
            &format!("{}{}", fixed, if variadic { "+" } else { "" }),
            args.len(),
            to_source_span(*call_span),
        ));
    }
    for (name, value) in lambda.params.required.iter().zip(args.iter()) {
        new_context.set_var(name, value.clone());
    }
    if let Some(rest) = &lambda.params.rest {
        let rest_args = &args[fixed..];
        let mut rest_list = Value::Nil;
        for item in rest_args.iter().rev() {
            rest_list = Value::Cons(Rc::new(ConsCell {
                car: item.clone(),
                cdr: rest_list,
            }));
        }
        new_context.set_var(rest, rest_list);
    }

    // Evaluate body in new context
    evaluate_ast_node(&lambda.body, &mut new_context)
}
