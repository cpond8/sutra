//! AST module for the Sutra language
//!
//! This module provides the core Abstract Syntax Tree types and builders
//! for representing Sutra expressions with source location tracking.

// ============================================================================
// IMPORTS
// ============================================================================

use serde::{Deserialize, Serialize};
use crate::runtime::path::Path;
use std::sync::Arc;

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

/// Represents a span in the source code.
///
/// All AST nodes carry a span for source tracking; enables better errors and explainability.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::Span;
/// let span = Span { start: 0, end: 5 };
/// assert_eq!(span.start, 0);
/// assert_eq!(span.end, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    // Optionally: line/col for richer error UX.
}

/// Wrapper for carrying source span information with any value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

/// Canonical AST node type with shared ownership for efficient macro expansion.
pub type AstNode = WithSpan<Arc<Expr>>;

/// The core AST node for Sutra expressions.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// let expr = Expr::Number(42.0, Span { start: 0, end: 2 });
/// assert_eq!(expr.span().start, 0);
/// assert_eq!(expr.span().end, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    // BREAKING: Now uses Vec<AstNode> for full span-carrying compliance
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
    /// Spread argument (e.g., ...args) for use in call position
    Spread(Box<AstNode>),
}

/// Parameter list for function definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamList {
    pub required: Vec<String>,
    pub rest: Option<String>,
    pub span: Span,
}

/// Errors that can occur during AST building
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SutraAstBuildError {
    InvalidShape { span: Span, message: String },
    UnknownRule { span: Span, rule: String },
    // ...
}

// ============================================================================
// PUBLIC API IMPLEMENTATION
// ============================================================================

impl Expr {
    /// Returns the span of this expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::{Expr, Span};
    /// let expr = Expr::Bool(true, Span { start: 1, end: 2 });
    /// assert_eq!(expr.span(), Span { start: 1, end: 2 });
    /// ```
    pub fn span(&self) -> Span {
        match self {
            Expr::List(_, span) => span.clone(),
            Expr::Symbol(_, span) => span.clone(),
            Expr::Path(_, span) => span.clone(),
            Expr::String(_, span) => span.clone(),
            Expr::Number(_, span) => span.clone(),
            Expr::Bool(_, span) => span.clone(),
            Expr::If { span, .. } => span.clone(),
            Expr::Quote(_, span) => span.clone(),
            Expr::ParamList(param_list) => param_list.span.clone(),
            Expr::Spread(expr) => expr.span.clone(),
        }
    }

    /// Converts this expression into a list of expressions if it is a list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::{Expr, Span, WithSpan};
    /// let expr = Expr::List(vec![], Span::default());
    /// assert_eq!(expr.into_list(), Some(vec![]));
    /// let expr2 = Expr::Number(1.0, Span::default());
    /// assert_eq!(expr2.into_list(), None);
    /// ```
    pub fn into_list(self) -> Option<Vec<AstNode>> {
        if let Expr::List(items, _) = self {
            Some(items)
        } else {
            None
        }
    }

    /// Pretty-prints the expression as a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::{Expr, Span};
    /// let expr = Expr::Symbol("foo".to_string(), Span::default());
    /// assert_eq!(expr.pretty(), "foo");
    /// let expr2 = Expr::Number(3.14, Span::default());
    /// assert_eq!(expr2.pretty(), "3.14");
    /// ```
    pub fn pretty(&self) -> String {
        match self {
            Expr::List(exprs, _) => Self::pretty_list(exprs),
            Expr::Symbol(s, _) => s.clone(),
            Expr::Path(p, _) => format!("(path {})", p.0.join(" ")),
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Number(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::If { condition, then_branch, else_branch, .. } => {
                Self::pretty_if(condition, then_branch, else_branch)
            }
            Expr::Quote(expr, _) => format!("'{}", expr.value.pretty()),
            Expr::ParamList(param_list) => Self::pretty_param_list(param_list),
            Expr::Spread(expr) => format!("...{}", expr.value.pretty()),
        }
    }

    // ------------------------------------------------------------------------
    // Pretty-printing helpers
    // ------------------------------------------------------------------------

    /// Helper for pretty-printing list expressions
    fn pretty_list(exprs: &[AstNode]) -> String {
        let inner = exprs
            .iter()
            .map(|e| e.value.pretty())
            .collect::<Vec<_>>()
            .join(" ");
        format!("({})", inner)
    }

    /// Helper for pretty-printing if expressions
    fn pretty_if(condition: &AstNode, then_branch: &AstNode, else_branch: &AstNode) -> String {
        format!(
            "(if {} {} {})",
            condition.value.pretty(),
            then_branch.value.pretty(),
            else_branch.value.pretty()
        )
    }

    /// Helper for pretty-printing parameter lists
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

// ============================================================================
// CONVERSIONS
// ============================================================================

impl From<crate::ast::value::Value> for Expr {
    fn from(val: crate::ast::value::Value) -> Self {
        use crate::ast::value::Value;
        match val {
            Value::Nil => Expr::List(vec![], Span::default()),
            Value::Number(n) => Expr::Number(n, Span::default()),
            Value::String(s) => Expr::String(s, Span::default()),
            Value::Bool(b) => Expr::Bool(b, Span::default()),
            Value::List(items) => {
                Expr::List(items.into_iter().map(|v| WithSpan { value: Arc::new(Expr::from(v)), span: Span::default() }).collect(), Span::default())
            },
            Value::Map(_) => Expr::List(vec![], Span::default()), // TODO: Map to a canonical representation if needed
            Value::Path(p) => Expr::Path(p, Span::default()),
        }
    }
}

// ============================================================================
// BUILDER INFRASTRUCTURE
// ============================================================================

/// Canonical AST builder trait for the modular pipeline.
pub trait SutraAstBuilder {
    fn build_ast(
        &self,
        cst: &crate::syntax::parser::SutraCstNode,
    ) -> Result<AstNode, SutraAstBuildError>;
}

/// Trivial AST builder for pipeline scaffolding (Sprint 2).
pub struct TrivialAstBuilder;

impl SutraAstBuilder for TrivialAstBuilder {
    fn build_ast(
        &self,
        cst: &crate::syntax::parser::SutraCstNode,
    ) -> Result<AstNode, SutraAstBuildError> {
        Ok(WithSpan {
            value: Arc::new(Expr::List(vec![], cst.span.clone())),
            span: cst.span.clone(),
        })
    }
}

/// Canonical AST builder for the modular pipeline (Sprint 3).
pub struct CanonicalAstBuilder;

impl SutraAstBuilder for CanonicalAstBuilder {
    fn build_ast(
        &self,
        cst: &crate::syntax::parser::SutraCstNode,
    ) -> Result<AstNode, SutraAstBuildError> {
        build_ast_from_cst(cst)
    }
}

// ============================================================================
// AST CONSTRUCTION HELPERS (INTERNAL)
// ============================================================================

/// Main CST to AST conversion function
fn build_ast_from_cst(
    cst: &crate::syntax::parser::SutraCstNode,
) -> Result<AstNode, SutraAstBuildError> {
    match cst.rule.as_str() {
        "program" | "list" => {
            map_cst_children_to_list(&cst.children, build_ast_from_cst, &cst.span)
        }
        "spread_arg" => build_spread_expr(cst),
        "number" => build_number_expr(cst),
        "boolean" => build_boolean_expr(cst),
        "string" => build_string_expr(cst),
        "symbol" => build_symbol_expr(cst),
        // Add more rules as needed (block, quote, define_form, etc.)
        _ => Err(SutraAstBuildError::UnknownRule {
            span: cst.span.clone(),
            rule: cst.rule.clone(),
        }),
    }
}

// ----------------------------------------------------------------------------
// Node-specific builders
// ----------------------------------------------------------------------------

/// Helper for building number expressions from CST
fn build_number_expr(cst: &crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError> {
    let text = cst_text(cst);
    let number = text.parse::<f64>()
        .map_err(|_| invalid_shape_error(&cst.span, format!("Invalid number: {}", text)))?;

    Ok(with_span(Arc::new(Expr::Number(number, cst.span.clone())), &cst.span))
}

/// Helper for building boolean expressions from CST
fn build_boolean_expr(cst: &crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError> {
    let text = cst_text(cst);
    let bool_value = match text.as_str() {
        "true" => true,
        "false" => false,
        _ => return Err(invalid_shape_error(&cst.span, format!("Invalid boolean: {}", text))),
    };

    Ok(with_span(Arc::new(Expr::Bool(bool_value, cst.span.clone())), &cst.span))
}

/// Helper for building string expressions from CST
fn build_string_expr(cst: &crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError> {
    let text = cst_text(cst);
    let unescaped = unescape_string(&text)
        .map_err(|msg| invalid_shape_error(&cst.span, msg))?;

    Ok(with_span(Arc::new(Expr::String(unescaped, cst.span.clone())), &cst.span))
}

/// Helper for building symbol expressions from CST
fn build_symbol_expr(cst: &crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError> {
    let text = cst_text(cst);
    Ok(with_span(Arc::new(Expr::Symbol(text, cst.span.clone())), &cst.span))
}

/// Helper for building spread expressions from CST
fn build_spread_expr(cst: &crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError> {
    // Guard clause: ensure exactly one child
    if cst.children.len() != 1 {
        return Err(invalid_shape_error(
            &cst.span,
            "Malformed spread_arg: expected one child (symbol)",
        ));
    }

    let symbol_expr = build_ast_from_cst(&cst.children[0])?;
    Ok(with_span(Arc::new(Expr::Spread(Box::new(symbol_expr))), &cst.span))
}

// ----------------------------------------------------------------------------
// Utility functions
// ----------------------------------------------------------------------------

/// Private combinator for mapping CST children to Expr::List
fn map_cst_children_to_list<F>(
    children: &[crate::syntax::parser::SutraCstNode],
    builder: F,
    span: &Span,
) -> Result<AstNode, SutraAstBuildError>
where
    F: FnMut(&crate::syntax::parser::SutraCstNode) -> Result<AstNode, SutraAstBuildError>,
{
    let exprs = children
        .iter()
        .map(builder)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WithSpan {
        value: Arc::new(Expr::List(exprs, span.clone())),
        span: span.clone(),
    })
}

/// Helper to construct an AstNode from an Arc<Expr> and a span.
fn with_span(expr: Arc<Expr>, span: &Span) -> AstNode {
    WithSpan {
        value: expr,
        span: span.clone(),
    }
}

/// Helper for creating invalid shape errors
fn invalid_shape_error(span: &Span, message: impl Into<String>) -> SutraAstBuildError {
    SutraAstBuildError::InvalidShape {
        span: span.clone(),
        message: message.into(),
    }
}

/// Extract text from CST node (placeholder implementation)
fn cst_text(cst: &crate::syntax::parser::SutraCstNode) -> String {
    // For leaf nodes, reconstruct text from span (in real impl, pass source)
    // Here, just use rule name as placeholder
    cst.rule.clone()
}

/// Unescape string content (placeholder implementation)
fn unescape_string(s: &str) -> Result<String, String> {
    // TODO: Implement real unescaping and validation
    Ok(s.to_string())
}

// ============================================================================
// MODULE EXPORTS
// ============================================================================

pub mod builder;
pub mod value;
