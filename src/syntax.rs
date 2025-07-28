//! Syntax module for the Sutra language
//!
//! This module provides the core Abstract Syntax Tree types for representing Sutra expressions with source location tracking.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::Path;

pub use crate::runtime::{ConsCell, Lambda, Value};

/// Represents a span in the source code.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Wrapper for carrying source span information with any value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

/// Canonical AST node type with shared ownership for efficient macro expansion.
pub type AstNode = Spanned<Arc<Expr>>;

/// The core AST node for Sutra expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    List(Vec<AstNode>, Span),
    Symbol(String, Span),
    Path(Path, Span),
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
    If {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Box<AstNode>,
        span: Span,
    },
    Quote(Box<AstNode>, Span),
    ParamList(ParamList),
    Spread(Box<AstNode>),
}

/// Parameter list for function definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamList {
    pub required: Vec<String>,
    pub rest: Option<String>,
    pub span: Span,
}

impl Expr {
    /// Returns the span of this expression.
    pub fn span(&self) -> Span {
        use Expr::*;
        match self {
            List(_, span)
            | Symbol(_, span)
            | Path(_, span)
            | String(_, span)
            | Number(_, span)
            | Bool(_, span)
            | If { span, .. }
            | Quote(_, span) => *span,
            ParamList(param_list) => param_list.span,
            Spread(expr) => expr.span,
        }
    }

    /// Converts this expression into a list of expressions if it is a list.
    pub fn into_list(self) -> Option<Vec<AstNode>> {
        if let Expr::List(items, _) = self {
            return Some(items);
        }
        None
    }

    /// Pretty-prints the expression as a string.
    pub fn pretty(&self) -> String {
        use Expr::*;
        match self {
            List(exprs, _) => Self::pretty_list(exprs),
            Symbol(s, _) => s.clone(),
            Path(p, _) => format!("(path {})", p.0.join(" ")),
            String(s, _) => format!("\"{}\"", s),
            Number(n, _) => n.to_string(),
            Bool(b, _) => b.to_string(),
            If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                format!(
                    "(if {} {} {})",
                    condition.value.pretty(),
                    then_branch.value.pretty(),
                    else_branch.value.pretty()
                )
            }
            Quote(expr, _) => format!("'{}", expr.value.pretty()),
            ParamList(param_list) => Self::pretty_param_list(param_list),
            Spread(expr) => format!("...{}", expr.value.pretty()),
        }
    }

    fn pretty_list(exprs: &[AstNode]) -> String {
        let inner = exprs
            .iter()
            .map(|e| e.value.pretty())
            .collect::<Vec<_>>()
            .join(" ");
        format!("({})", inner)
    }

    fn pretty_param_list(param_list: &ParamList) -> String {
        let mut s = String::from("(");
        let mut first = true;
        for req in &param_list.required {
            if !first {
                s.push(' ');
            }
            s.push_str(req);
            first = false;
        }
        if let Some(rest) = &param_list.rest {
            if !param_list.required.is_empty() {
                s.push_str(" . ");
            }
            s.push_str(rest);
        }
        s.push(')');
        s
    }

    /// Returns the type name of this AST node as a string (for diagnostics, debugging, and macro logic).
    pub fn type_name(&self) -> &'static str {
        match self {
            Expr::List(_, _) => "List",
            Expr::Symbol(_, _) => "Symbol",
            Expr::Path(_, _) => "Path",
            Expr::String(_, _) => "String",
            Expr::Number(_, _) => "Number",
            Expr::Bool(_, _) => "Bool",
            Expr::If { .. } => "If",
            Expr::Quote(_, _) => "Quote",
            Expr::ParamList(_) => "ParamList",
            Expr::Spread(_) => "Spread",
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

fn spanned(expr: Expr, span: Span) -> Spanned<Arc<Expr>> {
    Spanned {
        value: Arc::new(expr),
        span,
    }
}

/// Helper to check if a span is valid for a given source string.
pub fn assert_valid_span(span: Span, source: &str) {
    debug_assert!(
        span.start <= span.end && span.end <= source.len(),
        "Invalid span: {{start: {}, end: {}}} for source of length {}",
        span.start,
        span.end,
        source.len()
    );
}

/// Utility to construct an Expr from a Value, requiring a span.
pub fn expr_from_value_with_span(val: Value, span: Span) -> Result<Expr, String> {
    match val {
        Value::Nil => Ok(Expr::List(vec![], span)),
        Value::Number(n) => Ok(Expr::Number(n, span)),
        Value::String(s) => Ok(Expr::String(s, span)),
        Value::Bool(b) => Ok(Expr::Bool(b, span)),
        Value::Symbol(s) => Ok(Expr::Symbol(s, span)),
        Value::Quote(v) => {
            let quoted = expr_from_value_with_span(*v, span)?;
            Ok(Expr::Quote(Box::new(spanned(quoted, span)), span))
        }
        Value::Cons(cell) => {
            let mut items = Vec::new();
            let mut current = Value::Cons(cell);

            loop {
                match current {
                    Value::Cons(ref cell_repr) => {
                        let car_expr = expr_from_value_with_span(cell_repr.car().clone(), span)?;
                        items.push(spanned(car_expr, span));
                        current = cell_repr.cdr();
                    }
                    Value::Nil => break,
                    other => {
                        let cdr_expr = expr_from_value_with_span(other, span)?;
                        items.push(spanned(cdr_expr, span));
                        break;
                    }
                }
            }
            Ok(Expr::List(items, span))
        }
        Value::Map(map) => {
            let mut items = Vec::new();
            for (k, v) in map {
                let key_expr = Expr::String(k, span);
                let val_expr = expr_from_value_with_span(v, span)?;
                let pair = Expr::List(vec![spanned(key_expr, span), spanned(val_expr, span)], span);
                items.push(spanned(pair, span));
            }
            Ok(Expr::List(items, span))
        }
        Value::Path(p) => Ok(Expr::Path(p, span)),
        Value::Lambda(_) | Value::NativeFn(_) => Err(
            "Cannot convert function value to AST expression. This is a logic error.".to_string(),
        ),
    }
}

// WARNING: Do not use From<Value> for Expr in production code; use expr_from_value_with_span instead.
impl From<Value> for Expr {
    fn from(val: Value) -> Self {
        // This is only safe for tests or when span is irrelevant.
        match expr_from_value_with_span(val, Span::default()) {
            Ok(expr) => expr,
            Err(msg) => panic!("{}", msg),
        }
    }
}

// Re-export parser for backward compatibility
pub use crate::parser;
