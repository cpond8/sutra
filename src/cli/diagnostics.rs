//! Unified diagnostic presentation layer for Sutra errors.
//!
//! This module provides the `SutraDiagnostic` type which is responsible for all
//! error presentation logic, including colorization, source code snippets,
//! and span-based error highlighting.

use crate::syntax::error::SutraError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use termcolor::{Color, ColorSpec, WriteColor};

// === Constants ===

/// Number of lines of context to show before and after the error in code snippets.
const SNIPPET_CONTEXT_LINES: usize = 2;

// === Core Types ===

/// A diagnostic wrapper that combines a `SutraError` with source code context
/// to provide rich, formatted error presentation.
pub struct SutraDiagnostic<'a> {
    error: &'a SutraError,
    source: Option<&'a str>,
}

impl<'a> SutraDiagnostic<'a> {
    /// Creates a new diagnostic from an error and optional source code.
    ///
    /// # Arguments
    /// * `error` - The error to present
    /// * `source` - Optional source code for context and span highlighting
    pub fn new(error: &'a SutraError, source: Option<&'a str>) -> Self {
        Self { error, source }
    }
}

impl<'a> Display for SutraDiagnostic<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // Print the primary error message (from thiserror-derived Display)
        write!(f, "Error")?;

        // Add location information if span is available
        if let Some(span) = &self.error.span {
            let loc = format_location(span, self.source);
            write!(f, "{}", loc)?;
        }

        writeln!(f, ":")?;

        // Use the thiserror-derived Display implementation for the error message
        writeln!(f, "{}", self.error)?;

        // If both a span and source code are available, generate and display a code snippet with error highlighting.
        if let (Some(span), Some(source)) = (self.error.span.as_ref(), self.source) {
            if let Some(snippet) = generate_code_snippet(source, span) {
                write!(f, "\n{}", snippet)?;
            }
        }

        Ok(())
    }
}

// === Formatting Helpers ===

/// Formats the error location as a human-readable string, using line/col if possible.
fn format_location(span: &crate::ast::Span, source: Option<&str>) -> String {
    if let Some(source) = source {
        if let Some(((start_line, start_col), (end_line, end_col))) = span.byte_to_line_col(source) {
            return format!(" [at line {}, col {} to line {}, col {}]", start_line, start_col, end_line, end_col);
        }
    }
    format!(" [at {}-{}]", span.start, span.end)
}

/// Generates a multi-line code snippet with error highlighting.
fn generate_code_snippet(source: &str, span: &crate::ast::Span) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();

    // Convert byte positions to line/column positions
    let ((start_line, start_col), (end_line, end_col)) = span.byte_to_line_col(source)?;

    // Calculate display range (show context around the error)
    let display_start = start_line.saturating_sub(SNIPPET_CONTEXT_LINES).max(1);
    let display_end = (end_line + SNIPPET_CONTEXT_LINES).min(lines.len());

    let mut result = String::new();
    let line_num_width = display_end.to_string().len();

    for line_num in display_start..=display_end {
        let line_idx = line_num - 1;
        if line_idx >= lines.len() {
            continue;
        }
        let line = lines[line_idx];

        // Print line number and content
        result.push_str(&format!(
            "{:width$} | {}\n",
            line_num,
            line,
            width = line_num_width
        ));

        // Only add pointer line if this line contains the error
        if line_num < start_line || line_num > end_line {
            continue;
        }
        result.push_str(&format!("{:width$} | ", "", width = line_num_width));
        result.push_str(&pointer_line(
            line_num,
            start_line,
            end_line,
            start_col,
            end_col,
            line.chars().count(),
        ));
    }

    Some(result)
}

fn pointer_segment(s: &mut String, start: usize, end: usize, caret_at: usize) {
    for i in start..=end {
        if i == caret_at {
            s.push('^');
            continue;
        }
        s.push('-');
    }
}

fn pointer_line(
    line_num: usize,
    start_line: usize,
    end_line: usize,
    start_col: usize,
    end_col: usize,
    line_len: usize,
) -> String {
    let pointer_start = if line_num == start_line { start_col } else { 1 };
    let pointer_end = if line_num == end_line { end_col } else { line_len + 1 };
    let pointer_start = pointer_start.min(line_len + 1).max(1);
    let pointer_end = pointer_end.min(line_len + 1).max(pointer_start);

    let mut s = String::new();
    for _ in 1..pointer_start {
        s.push(' ');
    }

    match (line_num == start_line, line_num == end_line) {
        // Error starts and ends on this line
        (true, true) => {
            pointer_segment(&mut s, pointer_start, pointer_end, pointer_start);
            s.push_str(" err\n");
        }
        // Error begins on this line, continues to later lines
        (true, false) => {
            pointer_segment(&mut s, pointer_start, line_len, pointer_start);
            s.push_str(" err begins\n");
        }
        // Error ends on this line, started on earlier lines
        (false, true) => {
            pointer_segment(&mut s, 1, pointer_end, 1);
            s.push_str(" err ends\n");
        }
        // Error is ongoing (middle line)
        (false, false) => {
            s.push('|');
            s.push('\n');
        }
    }
    s
}

/// Generates a colored code snippet (for future colorization enhancement).
fn generate_code_snippet_colored(source: &str, span: &crate::ast::Span) -> Option<String> {
    // For now, use the same logic as the plain version
    // TODO: Implement syntax highlighting and colorization for code snippets in the future.
    generate_code_snippet(source, span)
}

// === Public API ===

/// Prints a diagnostic to standard error with colorization if supported.
///
/// This function attempts to display the diagnostic with colored output. If color output is not
/// available, it falls back to plain text. Use this to present errors to the user in a readable,
/// context-rich format.
///
/// # Example
/// ```
/// let diag = SutraDiagnostic::new(&error, Some(source));
/// print_diagnostic_to_stderr(&diag);
/// ```
pub fn print_diagnostic_to_stderr(diagnostic: &SutraDiagnostic) {
    use termcolor::{ColorChoice, StandardStream};
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    if let Err(_) = print_diagnostic_colored(&mut stderr, diagnostic) {
        // Fallback to plain text if color printing fails
        eprintln!("{}", diagnostic);
    }
}

// === Internal Color Printing ===

/// Prints a diagnostic with colors to any `WriteColor` implementation.
fn print_diagnostic_colored(
    writer: &mut impl WriteColor,
    diagnostic: &SutraDiagnostic,
) -> std::io::Result<()> {
    // Print "Error" in bold red
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    write!(writer, "Error")?;

    // Print location in regular text
    writer.reset()?;
    if let Some(span) = &diagnostic.error.span {
        let loc = format_location(span, diagnostic.source);
        write!(writer, "{}", loc)?;
    }
    writeln!(writer, ":")?;

    // Print error message in regular text
    writeln!(writer, "{}", diagnostic.error)?;

    // Print code snippet with highlighting
    if let (Some(span), Some(source)) = (&diagnostic.error.span, diagnostic.source) {
        if let Some(snippet) = generate_code_snippet_colored(source, span) {
            write!(writer, "\n{}", snippet)?;
        }
    }

    writer.reset()?;
    Ok(())
}
