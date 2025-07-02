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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    // Optionally: line/col for richer error UX.
}

use crate::path::Path;

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
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    List(Vec<Expr>, Span),
    Symbol(String, Span),
    Path(Path, Span),
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },
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
        }
    }

    /// Converts this expression into a list of expressions if it is a list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::ast::{Expr, Span};
    /// let expr = Expr::List(vec![], Span::default());
    /// assert_eq!(expr.into_list(), Some(vec![]));
    /// let expr2 = Expr::Number(1.0, Span::default());
    /// assert_eq!(expr2.into_list(), None);
    /// ```
    pub fn into_list(self) -> Option<Vec<Expr>> {
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
                    .map(|e| e.pretty())
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
                    condition.pretty(),
                    then_branch.pretty(),
                    else_branch.pretty()
                )
            }
        }
    }
}
