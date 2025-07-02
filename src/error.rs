use crate::ast::Span;

#[derive(Debug, Clone)]
pub struct EvalError {
    pub message: String,
    // The fully expanded code that was being executed when the error occurred.
    pub expanded_code: String,
    // The original, unexpanded code snippet from the author's source.
    // This is added during a second enrichment phase by the top-level runner.
    pub original_code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SutraErrorKind {
    Parse(String),
    Macro(String),
    Validation(String),
    Eval(EvalError),
    Io(String),
}

#[derive(Debug, Clone)]
pub struct SutraError {
    pub kind: SutraErrorKind,
    pub span: Option<Span>,
}

impl SutraError {
    // Helper to enrich the error with the original source code snippet.
    // This is part of the "two-phase error enrichment" pattern.
    pub fn with_source(mut self, source: &str) -> Self {
        if let Some(span) = &self.span {
            let original_code = source.get(span.start..span.end).map(|s| s.to_string());

            match &mut self.kind {
                SutraErrorKind::Eval(eval_error) => {
                    eval_error.original_code = original_code;
                }
                // This can be extended for other error kinds later.
                _ => {}
            }
        }
        self
    }
}

impl std::fmt::Display for SutraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            SutraErrorKind::Parse(s) => write!(f, "Parse Error: {}", s),
            SutraErrorKind::Macro(s) => write!(f, "Macro Error: {}", s),
            SutraErrorKind::Validation(s) => write!(f, "Validation Error: {}", s),
            SutraErrorKind::Io(s) => write!(f, "IO Error: {}", s),
            SutraErrorKind::Eval(e) => {
                writeln!(f, "Evaluation Error: {}", e.message)?;
                if let Some(suggestion) = &e.suggestion {
                    writeln!(f, "\nSuggestion: {}", suggestion)?;
                }
                if let Some(original) = &e.original_code {
                    writeln!(f, "\nOriginal Code:")?;
                    writeln!(f, "  {}", original)?;
                }
                writeln!(f, "\nExpanded Code:")?;
                write!(f, "  {}", e.expanded_code)
            }
        }
    }
}

impl std::error::Error for SutraError {}
