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
use crate::syntax::error::validation_error;
use crate::syntax::error::SutraError;
use crate::macros::MacroRegistry;
use crate::runtime::path::Path;

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
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_is;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("is?".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_is(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
pub fn expand_is(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "is?", "eq?")
}

/// Expands `(over? a b)` to `(gt? (core/get a) (core/get b))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_over;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("over?".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_over(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
pub fn expand_over(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "over?", "gt?")
}

/// Expands `(under? a b)` to `(lt? (core/get a) (core/get b))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_under;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("under?".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_under(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
pub fn expand_under(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "under?", "lt?")
}

/// Expands `(add! foo 1)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_add;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("add!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_add(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
pub fn expand_add(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_assignment_macro(expr, "add!", "+")
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_sub;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("sub!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_sub(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_inc;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("inc!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_inc(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_dec;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("dec!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_dec(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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

/// Expands `(set! foo 42)` to `(core/set! (path foo) 42)`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_set;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("set!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Number(42.0, Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_set(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_get;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("get".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_get(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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

/// Expands `(del! <path>)` to `(core/del! (path <...>))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::macros::std::expand_del;
/// let expr = WithSpan {
///     value: Expr::List(vec![
///         WithSpan { value: Expr::Symbol("del!".to_string(), Span::default()), span: Span::default() },
///         WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
///     ], Span::default()),
///     span: Span::default(),
/// };
/// let expanded = expand_del(&expr).unwrap();
/// assert!(matches!(expanded.value, Expr::List(_, _)));
/// ```
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

/// Expands `(if <cond> <then> <else>)` to a canonical `Expr::If` node.
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

/// Expands `(print <expr>)` to `(core/print <expr>)`.
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
