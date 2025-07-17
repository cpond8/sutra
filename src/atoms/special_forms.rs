use crate::ast::{AstNode, Expr};
use crate::ast::value::{Value, Lambda};
use crate::atoms::{SpecialFormAtomFn};
use crate::runtime::eval::evaluate_ast_node;
use crate::runtime::world::World;
use crate::SutraError;
use crate::err_msg;
use std::rc::Rc;

/// Implements the (lambda ...) special form.
pub const ATOM_LAMBDA: SpecialFormAtomFn = |args, _context, span| {
    if args.len() < 2 {
        return Err(err_msg!(Eval, "lambda expects a parameter list and at least one body expression"));
    }
    // Parse parameter list
    let param_list = match &*args[0].value {
        Expr::ParamList(pl) => pl.clone(),
        _ => return Err(err_msg!(Eval, "lambda: first argument must be a parameter list")),
    };

    // Validate parameter names (no duplicates, all symbols)
    let mut seen = std::collections::HashSet::new();
    for name in &param_list.required {
        if !seen.insert(name) {
            return Err(err_msg!(Eval, "lambda: duplicate parameter '{}'", name));
        }
    }
    if let Some(rest) = &param_list.rest {
        if !seen.insert(rest) {
            return Err(err_msg!(Eval, "lambda: duplicate variadic parameter '{}'", rest));
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
            std::iter::once(AstNode { value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *span)), span: *span })
                .chain(exprs)
                .collect(),
            *span,
        );
        Box::new(AstNode { value: std::sync::Arc::new(do_expr), span: *span })
    };

    Ok((Value::Lambda(Rc::new(Lambda { params: param_list, body })), _context.world.clone()))
};

/// Implements the (let ...) special form.
pub const ATOM_LET: SpecialFormAtomFn = |args, context, span| {
    if args.len() < 2 {
        return Err(err_msg!(Eval, "let expects a bindings list and at least one body expression"));
    }
    // Parse bindings
    let bindings = match &*args[0].value {
        Expr::List(pairs, _) => pairs,
        _ => return Err(err_msg!(Eval, "let: first argument must be a list of bindings")),
    };

    let mut new_context = context.clone_with_new_lexical_frame();

    // Evaluate and bind each (name value) pair in order
    for pair in bindings {
        let (name, value_expr) = match &*pair.value {
            Expr::List(items, _) if items.len() == 2 => {
                match &*items[0].value {
                    Expr::Symbol(name, _) => (name.clone(), &items[1]),
                    _ => return Err(err_msg!(Eval, "let: binding name must be a symbol")),
                }
            }
            _ => return Err(err_msg!(Eval, "let: each binding must be a (name value) pair")),
        };
        let (value, _) = evaluate_ast_node(value_expr, &mut new_context)?;
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
            std::iter::once(AstNode { value: std::sync::Arc::new(Expr::Symbol("do".to_string(), *span)), span: *span })
                .chain(exprs)
                .collect(),
            *span,
        );
        &AstNode { value: std::sync::Arc::new(do_expr), span: *span }
    };

    let (result, world) = evaluate_ast_node(body, &mut new_context)?;
    Ok((result, world))
};

/// Applies a Lambda value to arguments in the given evaluation context.
pub fn call_lambda(
    lambda: &Lambda,
    args: &[Value],
    context: &mut crate::runtime::eval::EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let mut new_context = context.clone_with_new_lexical_frame();

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
        return Err(err_msg!(Eval, "{}", msg));
    }
    for (name, value) in lambda.params.required.iter().zip(args.iter()) {
        new_context.set_lexical_var(name, value.clone());
    }
    if let Some(rest) = &lambda.params.rest {
        let rest_args = args[fixed..].to_vec();
        new_context.set_lexical_var(rest, Value::List(rest_args));
    }

    // Evaluate body in new context
    let (result, world) = evaluate_ast_node(&lambda.body, &mut new_context)?;
    Ok((result, world))
}