// All AST nodes carry a span for source tracking; enables better errors and explainability.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    // Optionally: line/col for richer error UX.
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    List(Vec<Expr>, Span),
    Symbol(String, Span),
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::List(_, span) => span.clone(),
            Expr::Symbol(_, span) => span.clone(),
            Expr::String(_, span) => span.clone(),
            Expr::Number(_, span) => span.clone(),
            Expr::Bool(_, span) => span.clone(),
        }
    }

    pub fn into_list(self) -> Option<Vec<Expr>> {
        if let Expr::List(items, _) = self {
            Some(items)
        } else {
            None
        }
    }

    // Utility: pretty printing, tree walking
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
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Number(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
        }
    }
}
