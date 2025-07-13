//! Handles all user-facing output for the CLI.
//!
//! This module is responsible for pretty-printing, colorizing output,
//! formatting errors, and generating JSON. By centralizing output logic here,
//! we ensure a consistent user experience across all commands.

// ============================================================================
// OUTPUT SINKS: OutputBuffer and StdoutSink implementations
// ============================================================================

use crate::atoms::OutputSink;
use crate::macros::MacroExpansionStep;
use difference::Changeset;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// OutputBuffer: collects output into a String for testing or programmatic capture.
pub struct OutputBuffer {
    pub buffer: String,
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.buffer
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for OutputBuffer {
    fn emit(&mut self, text: &str, _span: Option<&crate::ast::Span>) {
        if !self.buffer.is_empty() {
            self.buffer.push('\n');
        }
        self.buffer.push_str(text);
    }
}

/// StdoutSink: writes output to stdout for CLI and default runner use.
pub struct StdoutSink;

impl OutputSink for StdoutSink {
    fn emit(&mut self, text: &str, _span: Option<&crate::ast::Span>) {
        println!("{}", text);
    }
}

// ============================================================================
// CORE OUTPUT FUNCTIONS: User-facing CLI output utilities
// ============================================================================

/// Prints a macro expansion trace to the console with colored diffs.
pub fn print_trace(trace: &[MacroExpansionStep]) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut last_ast_str = String::new();

    for (i, step) in trace.iter().enumerate() {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        println!("--- Step {}: {} ---", i, step.macro_name);
        let _ = stdout.reset();

        let current_ast_str = step.output.value.pretty();

        if i == 0 {
            println!("{}", current_ast_str);
            last_ast_str = current_ast_str;
            println!();
            continue;
        }
        let changeset = Changeset::new(&last_ast_str, &current_ast_str, "\n");
        print_diff(&mut stdout, &changeset.diffs);
        last_ast_str = current_ast_str;
        println!();
    }
}

/// Pretty-prints an evaluation result to the console.
pub fn print_result<T: std::fmt::Debug>(result: &T) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    println!("Result: {:#?}", result);
    let _ = stdout.reset();
}

// ============================================================================
// PRIVATE HELPERS
// ============================================================================

fn print_diff(stdout: &mut StandardStream, diffs: &[difference::Difference]) {
    for diff in diffs {
        match diff {
            difference::Difference::Same(ref x) => {
                let _ = stdout.reset();
                println!(" {}", x);
            }
            difference::Difference::Add(ref x) => {
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                println!("+{}", x);
            }
            difference::Difference::Rem(ref x) => {
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                println!("-{}", x);
            }
        }
    }
}
