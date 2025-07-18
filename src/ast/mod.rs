//! AST module for the Sutra language
//!
//! This module provides the core Abstract Syntax Tree types for representing Sutra expressions with source location tracking.

use serde::{Deserialize, Serialize};
use crate::runtime::world::Path;
use std::sync::Arc;

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
    Define {
        name: String,
        params: ParamList,
        body: Box<AstNode>,
        span: Span,
    },
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
        match self {
            Expr::List(_, span) => *span,
            Expr::Symbol(_, span) => *span,
            Expr::Path(_, span) => *span,
            Expr::String(_, span) => *span,
            Expr::Number(_, span) => *span,
            Expr::Bool(_, span) => *span,
            Expr::If { span, .. } => *span,
            Expr::Quote(_, span) => *span,
            Expr::ParamList(param_list) => param_list.span,
            Expr::Define { span, .. } => *span,
            Expr::Spread(expr) => expr.span,
        }
    }

    /// Converts this expression into a list of expressions if it is a list.
    pub fn into_list(self) -> Option<Vec<AstNode>> {
        if let Expr::List(items, _) = self {
            Some(items)
        } else {
            None
        }
    }

    /// Pretty-prints the expression as a string.
    pub fn pretty(&self) -> String {
        match self {
            Expr::List(exprs, _) => Self::pretty_list(exprs),
            Expr::Symbol(s, _) => s.clone(),
            Expr::Path(p, _) => format!("(path {})", p.0.join(" ")),
            Expr::String(s, _) => format!("\"{s}\""),
            Expr::Number(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::If { condition, then_branch, else_branch, .. } => {
                Self::pretty_if(condition, then_branch, else_branch)
            }
            Expr::Quote(expr, _) => format!("'{}", expr.value.pretty()),
            Expr::ParamList(param_list) => Self::pretty_param_list(param_list),
            Expr::Define { name, params, body, .. } => {
                // Reconstruct the original define syntax: (define (name params...) body)
                let mut param_list_str = String::from("(");
                param_list_str.push_str(name);
                for param in &params.required {
                    param_list_str.push(' ');
                    param_list_str.push_str(param);
                }
                if let Some(rest) = &params.rest {
                    if !params.required.is_empty() {
                        param_list_str.push_str(" ...");
                    } else {
                        param_list_str.push_str(" ...");
                    }
                    param_list_str.push_str(rest);
                }
                param_list_str.push(')');
                format!("(define {} {})", param_list_str, body.value.pretty())
            }
            Expr::Spread(expr) => format!("...{}", expr.value.pretty()),
        }
    }

    fn pretty_list(exprs: &[AstNode]) -> String {
        let inner = exprs
            .iter()
            .map(|e| e.value.pretty())
            .collect::<Vec<_>>()
            .join(" ");
        format!("({inner})")
    }

    fn pretty_if(condition: &AstNode, then_branch: &AstNode, else_branch: &AstNode) -> String {
        format!(
            "(if {} {} {})",
            condition.value.pretty(),
            then_branch.value.pretty(),
            else_branch.value.pretty()
        )
    }

    fn pretty_param_list(param_list: &ParamList) -> String {
        let mut s = String::from("(");
        for (i, req) in param_list.required.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(req);
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
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

impl From<crate::ast::value::Value> for Expr {
    fn from(val: crate::ast::value::Value) -> Self {
        use crate::ast::value::Value;
        match val {
            Value::Nil => Expr::List(vec![], Span::default()),
            Value::Number(n) => Expr::Number(n, Span::default()),
            Value::String(s) => Expr::String(s, Span::default()),
            Value::Bool(b) => Expr::Bool(b, Span::default()),
            Value::List(items) => {
                Expr::List(items.into_iter().map(|v| Spanned { value: Arc::new(Expr::from(v)), span: Span::default() }).collect(), Span::default())
            },
            Value::Map(_) => Expr::List(vec![], Span::default()),
            Value::Path(p) => Expr::Path(p, Span::default()),
            Value::Lambda(_) => Expr::Symbol("<lambda>".to_string(), Span::default()),
        }
    }
}

pub mod value;
