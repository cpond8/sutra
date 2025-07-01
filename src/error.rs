use crate::ast::Span;

#[derive(Debug, Clone)]
pub enum SutraErrorKind {
    Parse(String),
    Macro(String),
    Validation(String),
    Eval(String),
    Io(String),
}

#[derive(Debug, Clone)]
pub struct SutraError {
    pub kind: SutraErrorKind,
    pub span: Option<Span>,
    // Optionally: cause/chain
}

impl std::fmt::Display for SutraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind_str = match &self.kind {
            SutraErrorKind::Parse(s) => format!("Parse Error: {}", s),
            SutraErrorKind::Macro(s) => format!("Macro Error: {}", s),
            SutraErrorKind::Validation(s) => format!("Validation Error: {}", s),
            SutraErrorKind::Eval(s) => format!("Evaluation Error: {}", s),
            SutraErrorKind::Io(s) => format!("IO Error: {}", s),
        };

        if let Some(span) = &self.span {
            write!(f, "{} at {}:{}", kind_str, span.start, span.end)
        } else {
            write!(f, "{}", kind_str)
        }
    }
}

impl std::error::Error for SutraError {}
