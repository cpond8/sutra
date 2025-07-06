use crate::ast::Span;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalError {
    pub message: String,
    // The fully expanded code that was being executed when the error occurred.
    pub expanded_code: String,
    // The original, unexpanded code snippet from the author's source.
    // This is added during a second enrichment phase by the top-level runner.
    pub original_code: Option<String>,
    pub suggestion: Option<String>,
}

/// The kind of error that occurred in Sutra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraErrorKind {
    Parse(String), // User-facing parse errors (malformed input, syntax error)
    Macro(String),
    Validation(String),
    Eval(EvalError),
    Io(String),
    // New error kinds for parser internal logic errors
    MalformedAst(String), // Unexpected AST structure, likely a bug or grammar mismatch
    InternalParse(String), // Internal parser state error, not user input
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

            if let SutraErrorKind::Eval(eval_error) = &mut self.kind {
                eval_error.original_code = original_code;
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
            SutraErrorKind::MalformedAst(s) => write!(f, "Malformed AST Error: {}", s),
            SutraErrorKind::InternalParse(s) => write!(f, "Internal Parse Error: {}", s),
        }
    }
}

impl std::error::Error for SutraError {}
