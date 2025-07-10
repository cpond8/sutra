// All AST nodes carry a span for source tracking; enables better errors and explainability.
/// Represents a span in the source code.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::Span;
/// let span = Span { start: 0, end: 5 };
/// assert_eq!(span.start, 0);
/// assert_eq!(span.end, 5);
/// ```
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    // Optionally: line/col for richer error UX.
}

use crate::runtime::path::Path;

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
    // BREAKING: Now uses Vec<WithSpan<Expr>> for full span-carrying compliance
    List(Vec<WithSpan<Expr>>, Span),
    Symbol(String, Span),
    Path(Path, Span),
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
    If {
        condition: Box<WithSpan<Expr>>,
        then_branch: Box<WithSpan<Expr>>,
        else_branch: Box<WithSpan<Expr>>,
        span: Span,
    },
    Quote(Box<WithSpan<Expr>>, Span),
    ParamList(ParamList),
    /// Spread argument (e.g., ...args) for use in call position
    Spread(Box<WithSpan<Expr>>),
}

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
    pub fn into_list(self) -> Option<Vec<WithSpan<Expr>>> {
        if let Expr::List(items, _) = self {
            Some(items)
        } else {
            None
        }
    }

    // Utility: pretty printing, tree walking
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
            Expr::List(exprs, _) => {
                let inner = exprs
                    .iter()
                    .map(|e| e.value.pretty())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({})", inner)
            }
            Expr::Symbol(s, _) => s.clone(),
            Expr::Path(p, _) => format!("(path {})", p.0.join(" ")),
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Number(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::If {
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
            Expr::Quote(expr, _) => format!("'{}", expr.value.pretty()),
            Expr::ParamList(param_list) => {
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
            Expr::Spread(expr) => format!("...{}", expr.value.pretty()),
        }
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
                Expr::List(items.into_iter().map(|v| WithSpan { value: Expr::from(v), span: Span::default() }).collect(), Span::default())
            },
            Value::Map(_) => Expr::List(vec![], Span::default()), // TODO: Map to a canonical representation if needed
            Value::Path(p) => Expr::Path(p, Span::default()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamList {
    pub required: Vec<String>,
    pub rest: Option<String>,
    pub span: Span,
}

/// Canonical AST builder trait for the modular pipeline.
pub trait SutraAstBuilder {
    fn build_ast(
        &self,
        cst: &crate::syntax::parser::SutraCstNode,
    ) -> Result<WithSpan<Expr>, SutraAstBuildError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SutraAstBuildError {
    InvalidShape { span: Span, message: String },
    UnknownRule { span: Span, rule: String },
    // ...
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

/// Trivial AST builder for pipeline scaffolding (Sprint 2).
pub struct TrivialAstBuilder;

impl SutraAstBuilder for TrivialAstBuilder {
    fn build_ast(
        &self,
        cst: &crate::syntax::parser::SutraCstNode,
    ) -> Result<WithSpan<Expr>, SutraAstBuildError> {
        Ok(WithSpan {
            value: Expr::List(vec![], cst.span.clone()),
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
    ) -> Result<WithSpan<Expr>, SutraAstBuildError> {
        build_ast_from_cst(cst)
    }
}

// Private combinator for mapping CST children to Expr::List
fn map_cst_children_to_list<F>(
    children: &[crate::syntax::parser::SutraCstNode],
    builder: F,
    span: &Span,
) -> Result<WithSpan<Expr>, SutraAstBuildError>
where
    F: FnMut(&crate::syntax::parser::SutraCstNode) -> Result<WithSpan<Expr>, SutraAstBuildError>,
{
    let exprs = children
        .iter()
        .map(builder)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WithSpan {
        value: Expr::List(exprs, span.clone()),
        span: span.clone(),
    })
}

fn build_ast_from_cst(
    cst: &crate::syntax::parser::SutraCstNode,
) -> Result<WithSpan<Expr>, SutraAstBuildError> {
    match cst.rule.as_str() {
        "program" | "list" => {
            map_cst_children_to_list(&cst.children, build_ast_from_cst, &cst.span)
        }
        "spread_arg" => {
            // Should have one child: a symbol
            if cst.children.len() != 1 {
                return Err(SutraAstBuildError::InvalidShape {
                    span: cst.span.clone(),
                    message: "Malformed spread_arg: expected one child (symbol)".to_string(),
                });
            }
            let symbol_expr = build_ast_from_cst(&cst.children[0])?;
            Ok(WithSpan {
                value: Expr::Spread(Box::new(symbol_expr)),
                span: cst.span.clone(),
            })
        }
        "number" => {
            let s = &cst_text(cst);
            let n = s
                .parse::<f64>()
                .map_err(|_| SutraAstBuildError::InvalidShape {
                    span: cst.span.clone(),
                    message: format!("Invalid number: {}", s),
                })?;
            Ok(WithSpan {
                value: Expr::Number(n, cst.span.clone()),
                span: cst.span.clone(),
            })
        }
        "boolean" => {
            let s = &cst_text(cst);
            let b = if s == "true" {
                true
            } else if s == "false" {
                false
            } else {
                Err(SutraAstBuildError::InvalidShape {
                    span: cst.span.clone(),
                    message: format!("Invalid boolean: {}", s),
                })?
            };
            Ok(WithSpan {
                value: Expr::Bool(b, cst.span.clone()),
                span: cst.span.clone(),
            })
        }
        "string" => {
            let s = &cst_text(cst);
            // Validate escape sequences (TODO: improve for all edge cases)
            let unescaped = unescape_string(s).map_err(|msg| SutraAstBuildError::InvalidShape {
                span: cst.span.clone(),
                message: msg,
            })?;
            Ok(WithSpan {
                value: Expr::String(unescaped, cst.span.clone()),
                span: cst.span.clone(),
            })
        }
        "symbol" => {
            let s = &cst_text(cst);
            Ok(WithSpan {
                value: Expr::Symbol(s.clone(), cst.span.clone()),
                span: cst.span.clone(),
            })
        }
        // Add more rules as needed (block, quote, define_form, etc.)
        _ => Err(SutraAstBuildError::UnknownRule {
            span: cst.span.clone(),
            rule: cst.rule.clone(),
        }),
    }
}

fn cst_text(cst: &crate::syntax::parser::SutraCstNode) -> String {
    // For leaf nodes, reconstruct text from span (in real impl, pass source)
    // Here, just use rule name as placeholder
    cst.rule.clone()
}

fn unescape_string(s: &str) -> Result<String, String> {
    // TODO: Implement real unescaping and validation
    Ok(s.to_string())
}

pub mod builder;
pub mod value;
