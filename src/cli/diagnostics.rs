//! Unified diagnostic presentation layer for Sutra errors.
//!
//! This module provides the `SutraDiagnostic` type which is responsible for all
//! error presentation logic, including colorization, source code snippets,
//! and span-based error highlighting.

use crate::syntax::error::SutraError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use termcolor::{Color, ColorSpec, WriteColor};
use unicode_segmentation::UnicodeSegmentation;

// === Constants ===

/// Number of lines of context to show before and after the error in code snippets.
const SNIPPET_CONTEXT_LINES: usize = 2;
const TAB_WIDTH: usize = 4;

// === Visual Layout Engine ===

/// Represents a line of source code with its original text and calculated visual length.
struct VisualLine {
    /// The original text of the line, may include a trailing newline.
    text: String,
    /// The visual length of the line, with tabs expanded.
    visual_len: usize,
}

impl VisualLine {
    /// Creates a new `VisualLine`, calculating its visual length from the text.
    /// The visual length accounts for tab expansion but ignores any trailing newline.
    fn new(text: &str) -> Self {
        let mut visual_col = 1;
        let content = text.trim_end_matches('\n');
        for grapheme in content.graphemes(true) {
            if grapheme == "\t" {
                visual_col += TAB_WIDTH - ((visual_col - 1) % TAB_WIDTH);
            } else {
                visual_col += 1;
            }
        }
        Self {
            text: text.to_string(),
            visual_len: visual_col - 1,
        }
    }
}

/// Manages the visual layout of a multi-line source string.
struct VisualLayout {
    lines: Vec<VisualLine>,
}

impl VisualLayout {
    /// Creates a new `VisualLayout` from a source string, splitting it into
    /// `VisualLine`s while preserving newline characters in the line text.
    fn new(source: &str) -> Self {
        if source.is_empty() {
            return Self { lines: vec![VisualLine::new("")] };
        }
        let mut lines = Vec::new();
        let mut start = 0;
        for (i, _) in source.match_indices('\n') {
            lines.push(VisualLine::new(&source[start..=i]));
            start = i + 1;
        }
        if start < source.len() {
            lines.push(VisualLine::new(&source[start..]));
        }
        Self { lines }
    }

    /// Gets the `VisualLine` for a given 1-based line number.
    fn get_line(&self, line_num: usize) -> Option<&VisualLine> {
        self.lines.get(line_num - 1)
    }
}

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
    let layout = VisualLayout::new(source);
    let ((start_line, start_col), (end_line, end_col)) = span.byte_to_line_col(source)?;

    let display_start = start_line.saturating_sub(SNIPPET_CONTEXT_LINES).max(1);
    let display_end = (end_line + SNIPPET_CONTEXT_LINES).min(layout.lines.len());

    let mut result = String::new();
    let line_num_width = display_end.to_string().len();

    for line_num in display_start..=display_end {
        if let Some(visual_line) = layout.get_line(line_num) {
            let display_text = visual_line
                .text
                .trim_end_matches('\n')
                .replace('\t', &" ".repeat(TAB_WIDTH));

            result.push_str(&format!(
                "{:width$} | {}\n",
                line_num, display_text, width = line_num_width
            ));

            if line_num >= start_line && line_num <= end_line {
                result.push_str(&format!("{:width$} | ", "", width = line_num_width));
                result.push_str(&pointer_line(
                    line_num,
                    start_line,
                    end_line,
                    start_col,
                    end_col,
                    visual_line.visual_len,
                ));
            }
        }
    }

    Some(result)
}

/// Generates the pointer line for a diagnostic snippet.
///
/// This function constructs the line of carets (`^`), dashes (`-`), and markers
/// that highlight the error span. It handles single-line, multi-line start,
/// multi-line end, and intermediate multi-line cases.
///
/// # Arguments
/// * `line_num` - The current line number being processed.
/// * `start_line`, `end_line` - The start and end lines of the error span.
/// * `start_col`, `end_col` - The visual start and end columns of the error span.
/// * `visual_line_len` - The total visual length of the current line.
///
/// # Returns
/// A string containing the formatted pointer line.
fn pointer_line(
    line_num: usize,
    start_line: usize,
    end_line: usize,
    start_col: usize,
    end_col: usize,
    visual_line_len: usize,
) -> String {
    let mut s = String::new();
    let is_start_line = line_num == start_line;
    let is_end_line = line_num == end_line;

    match (is_start_line, is_end_line) {
        // Case 1: Single-line span. `^----`
        (true, true) => {
            s.push_str(&" ".repeat(start_col.saturating_sub(1)));
            s.push('^');
            let num_dashes = end_col.saturating_sub(start_col);
            s.push_str(&"-".repeat(num_dashes));
            s.push_str(" here\n");
        }
        // Case 2: Start of a multi-line span. `^-----...`
        (true, false) => {
            s.push_str(&" ".repeat(start_col.saturating_sub(1)));
            s.push('^');
            let num_dashes = visual_line_len.saturating_sub(start_col);
            s.push_str(&"-".repeat(num_dashes));
            s.push_str(" here\n");
        }
        // Case 3: End of a multi-line span. `...----^`
        (false, true) => {
            let num_dashes = end_col.saturating_sub(1);
            s.push_str(&"-".repeat(num_dashes));
            s.push('^');
            s.push_str(" here\n");
        }
        // Case 4: A line in the middle of a multi-line span. `|`
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
