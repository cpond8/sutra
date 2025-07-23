//! AST module for the Sutra language
//!
//! This module provides the core Abstract Syntax Tree types for representing Sutra expressions with source location tracking.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{Path, Value};

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
            If { condition, then_branch, else_branch, .. } => {
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
        let inner = exprs.iter().map(|e| e.value.pretty()).collect::<Vec<_>>().join(" ");
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

impl From<Value> for Expr {
    fn from(val: Value) -> Self {
        use Value::*;
        match val {
            Nil | Map(_) => Expr::List(vec![], Span::default()),
            Number(n) => Expr::Number(n, Span::default()),
            String(s) => Expr::String(s, Span::default()),
            Bool(b) => Expr::Bool(b, Span::default()),
            List(items) => Expr::List(
                items
                    .into_iter()
                    .map(|v| Spanned {
                        value: Arc::new(Expr::from(v)),
                        span: Span::default(),
                    })
                    .collect(),
                Span::default(),
            ),
            Path(p) => Expr::Path(p, Span::default()),
            Lambda(_) => Expr::Symbol("<lambda>".to_string(), Span::default()),
        }
    }
}

impl<T> Spanned<T> {
    /// Returns the type name of the inner value if it has a type_name() method.
    pub fn type_name(&self) -> &'static str
    where
        T: std::ops::Deref,
        <T as std::ops::Deref>::Target: TypeNameTrait,
    {
        self.value.type_name()
    }
}

/// Trait for types that provide a type_name method.
pub trait TypeNameTrait {
    fn type_name(&self) -> &'static str;
}

impl TypeNameTrait for Expr {
    fn type_name(&self) -> &'static str {
        self.type_name()
    }
}

pub mod value;
