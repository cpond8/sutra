//! # Sutra Standard Macro Library
//!
//! This module is the sole authority on path canonicalization and provides
//! the core, author-facing macros that expand into simpler, canonical ASTs.
//!
//! ## Core Responsibility: Path Canonicalization
//!
//! The primary role of this module is to convert user-friendly path syntax
//! (e.g., `player.score` or `(player score)`) into a canonical `Expr::Path`
//! node. This is the only place in the entire engine where path syntax is parsed.

use crate::ast::{Expr, WithSpan};
use crate::macros::MacroRegistry;
use crate::runtime::path::Path;
use crate::syntax::error::validation_error;
use crate::syntax::error::SutraError;

// ---
// Registry
// ---

/// Registers all standard macros in the given registry.
pub fn register_std_macros(registry: &mut MacroRegistry) {
    // Core macros
    registry.register("set!", expand_set);
    registry.register("get", expand_get);
    registry.register("del!", expand_del);

    // Conditional macros
    registry.register("if", expand_if);

    // Predicate macros
    registry.register("is?", expand_is);
    registry.register("over?", expand_over);
    registry.register("under?", expand_under);
    // Assignment macros
    registry.register("add!", expand_add);
    registry.register("sub!", expand_sub);
    registry.register("inc!", expand_inc);
    registry.register("dec!", expand_dec);
    // Standard macros like cond are now loaded from macros.sutra at startup.
    registry.register("print", expand_print);
}

// ---
// Path Canonicalization: The Single Source of Truth
// ---

/// Converts a user-facing expression (`Symbol` or `List`) into a canonical `Path`.
/// This is the only function in the engine that understands path syntax.
fn expr_to_path(expr: &WithSpan<Expr>) -> Result<Path, SutraError> {
    match &expr.value {
        // Dotted symbol syntax: `player.score`
        Expr::Symbol(s, _) => Ok(Path(s.split('.').map(String::from).collect())),
        // List syntax: `(player score)`
        Expr::List(items, _) => {
            let segments: Result<Vec<_>, _> = items
                .iter()
                .map(|item| match &item.value {
                    Expr::Symbol(s, _) | Expr::String(s, _) => Ok(s.clone()),
                    _ => Err(validation_error(
                        "Path lists can only contain symbols or strings.",
                        Some(item.span.clone()),
                    )),
                })
                .collect();
            Ok(Path(segments?))
        }
        _ => Err(validation_error(
            "Invalid path format: expected a symbol or a list.",
            Some(expr.span.clone()),
        )),
    }
}

/// A helper to wrap a path-like expression in a `(core/get ...)` call.
/// If the expression is a valid path, it's converted to an `Expr::Path` and
/// wrapped in `(core/get ...)`. Otherwise, it's returned as-is.
fn wrap_in_get(expr: &WithSpan<Expr>) -> WithSpan<Expr> {
    if let Ok(path) = expr_to_path(expr) {
        let get_symbol = WithSpan {
            value: Expr::Symbol("core/get".to_string(), expr.span.clone()),
            span: expr.span.clone(),
        };
        let path_expr = WithSpan {
            value: Expr::Path(path, expr.span.clone()),
            span: expr.span.clone(),
        };
        WithSpan {
            value: Expr::List(vec![get_symbol, path_expr], expr.span.clone()),
            span: expr.span.clone(),
        }
    } else {
        expr.clone()
    }
}

// ---
// Macro Helpers
// ---

/// Validates that the given expression is a list with the expected number of arguments.
/// Returns the items and span if valid, or a SutraError otherwise.
fn expect_list_with_n_args<'a>(
    expr: &'a WithSpan<Expr>,
    n: usize,
    macro_name: &str,
) -> Result<(&'a [WithSpan<Expr>], &'a crate::ast::Span), SutraError> {
    match &expr.value {
        Expr::List(items, span) if items.len() == n => Ok((items, span)),
        Expr::List(items, span) => Err(validation_error(
            format!(
                "Macro '{}' expects {} arguments, but got {}",
                macro_name,
                n - 1,
                items.len() - 1
            ),
            Some(span.clone()),
        )),
        _ => Err(validation_error(
            format!("Macro '{}' can only be applied to a list.", macro_name),
            Some(expr.span.clone()),
        )),
    }
}

/// Helper for binary predicate macros like `is?`, `over?`, etc.
fn create_binary_predicate_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    atom_name: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, macro_name)?;
    let atom_symbol = WithSpan {
        value: Expr::Symbol(atom_name.to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let arg1 = wrap_in_get(&items[1]);
    let arg2 = wrap_in_get(&items[2]);
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, arg1, arg2], span.clone()),
        span: span.clone(),
    })
}

/// Helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    op_symbol: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, macro_name)?;
    let set_symbol = WithSpan {
        value: Expr::Symbol("core/set!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    let value_arg = items[2].clone();
    let atom_symbol = WithSpan {
        value: Expr::Symbol(op_symbol.to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let inner_expr = WithSpan {
        value: Expr::List(
            vec![atom_symbol, wrap_in_get(path_arg), value_arg],
            span.clone(),
        ),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

// ---
// Standard Macros
// ---

/// Expands `(is? a b)` to `(eq? (core/get a) (core/get b))`.
///
/// Usage: (is? a b)
/// Example:
///   (is? foo bar) ; expands to (eq? (core/get foo) (core/get bar))
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_is(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "is?", "eq?")
}

/// Expands `(over? a b)` to `(gt? (core/get a) (core/get b))`.
///
/// Usage: (over? a b)
/// Example:
///   (over? foo bar) ; expands to (gt? (core/get foo) (core/get bar))
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_over(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "over?", "gt?")
}

/// Expands `(under? a b)` to `(lt? (core/get a) (core/get b))`.
///
/// Usage: (under? a b)
/// Example:
///   (under? foo bar) ; expands to (lt? (core/get foo) (core/get bar))
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_under(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "under?", "lt?")
}

/// Expands `(add! foo 1)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
///
/// Usage: (add! foo 1)
/// Example:
///   (add! foo 1) ; expands to (core/set! (path foo) (+ (core/get foo) 1))
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_add(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_assignment_macro(expr, "add!", "+")
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
///
/// Usage: (sub! foo 1)
/// Example:
///   (sub! foo 1) ; expands to (core/set! (path foo) (- (core/get foo) 1))
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_sub(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, "sub!")?;
    let set_symbol = WithSpan {
        value: Expr::Symbol("core/set!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    let value_arg = items[2].clone();
    let atom_symbol = WithSpan {
        value: Expr::Symbol("-".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let inner_expr = WithSpan {
        value: Expr::List(
            vec![atom_symbol, wrap_in_get(path_arg), value_arg],
            span.clone(),
        ),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(inc! foo)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
///
/// Usage: (inc! foo)
/// Example:
///   (inc! foo) ; expands to (core/set! (path foo) (+ (core/get foo) 1))
///
/// # Caveats
/// Only works for one argument.
pub fn expand_inc(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "inc!")?;
    let set_symbol = WithSpan {
        value: Expr::Symbol("core/set!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    let add_symbol = WithSpan {
        value: Expr::Symbol("+".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let one = WithSpan {
        value: Expr::Number(1.0, items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let inner_expr = WithSpan {
        value: Expr::List(vec![add_symbol, wrap_in_get(path_arg), one], span.clone()),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(dec! foo)` to `(core/set! (path foo) (- (core/get foo) 1))`.
///
/// Usage: (dec! foo)
/// Example:
///   (dec! foo) ; expands to (core/set! (path foo) (- (core/get foo) 1))
///
/// # Caveats
/// Only works for one argument.
pub fn expand_dec(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "dec!")?;
    let set_symbol = WithSpan {
        value: Expr::Symbol("core/set!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    let sub_symbol = WithSpan {
        value: Expr::Symbol("-".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let one = WithSpan {
        value: Expr::Number(1.0, items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let inner_expr = WithSpan {
        value: Expr::List(vec![sub_symbol, wrap_in_get(path_arg), one], span.clone()),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(set! foo bar)` to `(core/set! (path foo) bar)`.
///
/// Usage: (set! foo bar)
/// Example:
///   (set! foo bar) ; expands to (core/set! (path foo) bar)
///
/// # Caveats
/// Only works for two arguments.
pub fn expand_set(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, "set!")?;
    let atom_symbol = WithSpan {
        value: Expr::Symbol("core/set!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    let value_arg = items[2].clone();
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path, value_arg], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(get foo)` to `(core/get (path foo))`.
///
/// Usage: (get foo)
/// Example:
///   (get foo) ; expands to (core/get (path foo))
///
/// # Caveats
/// Only works for one argument.
pub fn expand_get(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "get")?;
    let atom_symbol = WithSpan {
        value: Expr::Symbol("core/get".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(del! foo)` to `(core/del! (path foo))`.
///
/// Usage: (del! foo)
/// Example:
///   (del! foo) ; expands to (core/del! (path foo))
///
/// # Caveats
/// Only works for one argument.
pub fn expand_del(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "del!")?;
    let atom_symbol = WithSpan {
        value: Expr::Symbol("core/del!".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    let path_arg = &items[1];
    let canonical_path = WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path], span.clone()),
        span: span.clone(),
    })
}

/// Expands `(if cond then else)` to a canonical conditional form.
///
/// Usage: (if cond then else)
/// Example:
///   (if (eq? x 1) (print "yes") (print "no"))
///
/// # Caveats
/// Only works for three arguments.
pub fn expand_if(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 4, "if")?;
    Ok(WithSpan {
        value: Expr::If {
            condition: Box::new(items[1].clone()),
            then_branch: Box::new(items[2].clone()),
            else_branch: Box::new(items[3].clone()),
            span: span.clone(),
        },
        span: span.clone(),
    })
}

/// Expands `(print x)` to a canonical print form.
///
/// Usage: (print x)
/// Example:
///   (print x) ; expands to (print x)
///
/// # Caveats
/// Only works for one argument.
pub fn expand_print(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "print")?;
    let atom_symbol = WithSpan {
        value: Expr::Symbol("core/print".to_string(), items[0].span.clone()),
        span: items[0].span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, items[1].clone()], span.clone()),
        span: span.clone(),
    })
}
