use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::prelude::*;
use crate::{
    ast::value::Lambda,
    ast::ParamList,
    atoms::helpers::{validate_special_form_arity, validate_special_form_min_arity, AtomResult},
    errors,
    runtime::{eval, world::Path},
};

fn find_and_capture_free_variables(
    body: &AstNode,
    params: &ParamList,
    context: &eval::EvaluationContext,
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
        if let Some(value) = context.get_lexical_var(&symbol) {
            captured_env.insert(symbol, value.clone());
        }
    }

    captured_env
}

/// Implements the (define ...) special form for global bindings.
pub const ATOM_DEFINE: NativeLazyFn = |args, context, span| {
    // 1. Validate arity: (define <name> <value>)
    validate_special_form_arity(args, 2, "define")?;

    let name_expr = &args[0];
    let value_expr = &args[1];

    // 2. Handle variable definition: (define my-var 100)
    if let Expr::Symbol(name, _) = &*name_expr.value {
        // Evaluate the value expression in the current context.
        let value = eval::evaluate_ast_node(value_expr, context)?;

        // Create a path from the name and update the world state.
        let path = Path(vec![name.clone()]);
        context.world.borrow_mut().set(&path, value.clone());

        return Ok(value);
    }

    // 3. Handle function definition: (define (my-func x) (+ x 1))
    if let Expr::List(items, list_span) = &*name_expr.value {
        if items.is_empty() {
            return Err(errors::runtime_general(
                "define: function definition requires a name",
                context.current_file(),
                context.current_source(),
                context.span_for_span(*span),
            ));
        }

        // The first item is the function name.
        let function_name = match &*items[0].value {
            Expr::Symbol(s, _) => s.clone(),
            _ => {
                return Err(errors::runtime_general(
                    "define: function name must be a symbol",
                    context.current_file(),
                    context.current_source(),
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
                        return Err(errors::runtime_general(
                            "define: spread parameter must be a symbol",
                            context.current_file(),
                            context.current_source(),
                            context.span_for_node(spread_expr),
                        ));
                    }
                }
                _ => {
                    return Err(errors::runtime_general(
                        "define: function parameters must be symbols",
                        context.current_file(),
                        context.current_source(),
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

        let body = Box::new(value_expr.clone());
        let captured_env = find_and_capture_free_variables(&body, &params, context);

        let lambda = Value::Lambda(Rc::new(Lambda {
            params,
            body,
            captured_env,
        }));

        // Create a path from the function name and update the world state.
        let path = Path(vec![function_name.clone()]);
        context.world.borrow_mut().set(&path, lambda.clone());

        return Ok(lambda);
    }

    // 4. If the first argument is not a symbol or a list, it's an error.
    Err(errors::runtime_general(
        "define: first argument must be a symbol or a list for function definition",
        context.current_file(),
        context.current_source(),
        context.span_for_node(name_expr),
    ))
};

/// Implements the (if ...) special form with lazy evaluation.
pub const ATOM_IF: NativeLazyFn = |args, context, _span| {
    validate_special_form_arity(args, 3, "if")?;
    let condition = &args[0];
    let then_branch = &args[1];
    let else_branch = &args[2];

    let is_true = eval::evaluate_condition_as_bool(condition, context)?;
    let branch = if is_true { then_branch } else { else_branch };
    eval::evaluate_ast_node(branch, context)
};

/// Implements the (lambda ...) special form.
pub const ATOM_LAMBDA: NativeLazyFn = |args, context, span| {
    validate_special_form_min_arity(args, 2, "lambda")?;
    // Parse parameter list
    let param_list = match &*args[0].value {
        Expr::ParamList(pl) => pl.clone(),
        _ => {
            return Err(errors::runtime_general(
                "lambda: first argument must be a parameter list",
                context.current_file(),
                context.current_source(),
                context.span_for_span(*span),
            ));
        }
    };

    // Validate parameter names (no duplicates, all symbols)
    let mut seen = std::collections::HashSet::new();
    for name in &param_list.required {
        if !seen.insert(name) {
            return Err(errors::runtime_general(
                format!("lambda: duplicate parameter '{}'", name),
                context.current_file(),
                context.current_source(),
                context.span_for_span(*span),
            ));
        }
    }
    if let Some(rest) = &param_list.rest {
        if !seen.insert(rest) {
            return Err(errors::runtime_general(
                format!("lambda: duplicate variadic parameter '{}'", rest),
                context.current_file(),
                context.current_source(),
                context.span_for_span(*span),
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

    let captured_env = find_and_capture_free_variables(&body, &param_list, context);

    Ok(Value::Lambda(Rc::new(Lambda {
        params: param_list,
        body,
        captured_env,
    })))
};

/// Implements the (let ...) special form.
pub const ATOM_LET: NativeLazyFn = |args, context, span| {
    validate_special_form_min_arity(args, 2, "let")?;
    // Parse bindings
    let bindings = match &*args[0].value {
        Expr::List(pairs, _) => pairs,
        _ => {
            return Err(errors::runtime_general(
                "let: first argument must be a list of bindings",
                context.current_file(),
                context.current_source(),
                context.span_for_span(*span),
            ));
        }
    };

    let mut new_context = context.clone_with_new_lexical_frame();

    // Evaluate and bind each (name value) pair in order
    for pair in bindings {
        let (name, value_expr) = match &*pair.value {
            Expr::List(items, _) if items.len() == 2 => match &*items[0].value {
                Expr::Symbol(name, _) => (name.clone(), &items[1]),
                _ => {
                    return Err(errors::runtime_general(
                        "let: binding name must be a symbol",
                        context.current_file(),
                        context.current_source(),
                        context.span_for_span(*span),
                    ));
                }
            },
            _ => {
                return Err(errors::runtime_general(
                    "let: each binding must be a (name value) pair",
                    context.current_file(),
                    context.current_source(),
                    context.span_for_span(*span),
                ));
            }
        };
        let value = eval::evaluate_ast_node(value_expr, &mut new_context)?;
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

    eval::evaluate_ast_node(body, &mut new_context)
};

/// Applies a Lambda value to arguments in the given evaluation context.
pub fn call_lambda(
    lambda: &Lambda,
    args: &[Value],
    context: &mut eval::EvaluationContext,
) -> AtomResult {
    let mut new_context = eval::EvaluationContext {
        world: Rc::clone(&context.world),
        output: context.output.clone(),
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.depth + 1,
        lexical_env: vec![lambda.captured_env.clone()],
        test_file: context.test_file.clone(),
        test_name: context.test_name.clone(),
    };
    new_context.lexical_env.push(HashMap::new());

    // Bind parameters in the new top frame
    let fixed = lambda.params.required.len();
    let variadic = lambda.params.rest.is_some();
    if (!variadic && args.len() != fixed) || (variadic && args.len() < fixed) {
        let msg = format!(
            "lambda: expected {}{} arguments, got {}",
            fixed,
            if variadic { "+" } else { "" },
            args.len()
        );
        return Err(errors::runtime_general(
            msg,
            context.current_file(),
            context.current_source(),
            context.span_for_span(Span::default()),
        ));
    }
    for (name, value) in lambda.params.required.iter().zip(args.iter()) {
        new_context.set_lexical_var(name, value.clone());
    }
    if let Some(rest) = &lambda.params.rest {
        let rest_args = args[fixed..].to_vec();
        new_context.set_lexical_var(rest, Value::List(rest_args));
    }

    // Evaluate body in new context
    eval::evaluate_ast_node(&lambda.body, &mut new_context)
}
