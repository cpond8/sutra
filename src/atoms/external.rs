//! External system interface atoms for the Sutra language.
//!
//! This module provides atoms that interact with external systems, breaking the
//! pure functional model through I/O operations and randomness.
//!
//! ## Atoms Provided
//!
//! - **I/O Operations**: `print`, `println`, `output`
//! - **Randomness**: `rand`
//!
//! ## Design Notes
//!
//! These atoms have side effects and may produce non-deterministic results.
//! The PRNG used for `rand` is seedable for testing purposes.

use crate::{
    errors::{to_source_span, ErrorReporting},
    prelude::*,
    runtime::{evaluate_ast_node, SpannedValue},
};

// ============================================================================
// OUTPUT TYPES - Generic output handling for CLI and testing
// ============================================================================

/// OutputBuffer: collects output into a String for testing or programmatic capture.
pub struct EngineOutputBuffer {
    pub buffer: String,
}

impl EngineOutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.buffer
    }
}

impl Default for EngineOutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::atoms::OutputSink for EngineOutputBuffer {
    fn emit(&mut self, text: &str, _span: Option<&Span>) {
        self.buffer.push_str(text);
    }
}

/// StdoutSink: writes output to stdout for CLI and default runner use.
pub struct EngineStdoutSink;

impl crate::atoms::OutputSink for EngineStdoutSink {
    fn emit(&mut self, text: &str, _span: Option<&Span>) {
        println!("{text}");
    }
}

// ============================================================================
// I/O OPERATIONS
// ============================================================================

/// Prints concatenated arguments to the output sink.
///
/// Usage: (print <value>...)
///   - <value>: Any number of values to print
///
///   Returns: Nil. Emits output without trailing newline.
///
/// Examples:
///   (print "hello")     ; outputs "hello"
///   (print "hello" 123 true)  ; outputs "hello123true"
pub const ATOM_PRINT: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Err(context.arity_mismatch("at least 1", 0, to_source_span(*call_span)));
    }

    let mut output = String::new();
    for arg in args {
        let spanned_val = evaluate_ast_node(arg, context)?;
        output.push_str(&spanned_val.value.to_string());
    }

    context.output.borrow_mut().emit(&output, Some(call_span));
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Prints concatenated arguments to the output sink with a trailing newline.
///
/// Usage: (println <value>...)
///   - <value>: Any number of values to print (zero or more)
///
///   Returns: Nil. Emits output with newline.
///
/// Examples:
///   (println "hello")     ; outputs "hello\n"
///   (println "hello" 123 true)  ; outputs "hello123true\n"
///   (println)  ; prints just a newline
pub const ATOM_PRINTLN: NativeFn = |args, context, call_span| {
    let mut output = String::new();
    for arg in args {
        let spanned_val = evaluate_ast_node(arg, context)?;
        output.push_str(&spanned_val.value.to_string());
    }
    output.push('\n');

    context.output.borrow_mut().emit(&output, Some(call_span));
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Emits output to the output sink (alias for print).
///
/// Usage: (output <value>)
///   - <value>: Any value
///
///   Returns: Nil. Emits output.
///
/// Example:
///   (output "hello")
pub const ATOM_OUTPUT: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }
    let spanned_val = evaluate_ast_node(&args[0], context)?;
    context
        .output
        .borrow_mut()
        .emit(&spanned_val.value.to_string(), Some(&spanned_val.span));
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

// ============================================================================
// RANDOMNESS OPERATIONS
// ============================================================================

/// Generates a pseudo-random number between 0.0 (inclusive) and 1.0 (exclusive).
///
/// Usage: (rand)
///   - No arguments
///
///   Returns: Number (pseudo-random float between 0.0 and 1.0)
///
/// Example:
///   (rand) ; => 0.7234567 (example)
pub const ATOM_RAND: NativeFn = |args, context, call_span| {
    if !args.is_empty() {
        return Err(context.arity_mismatch("0", args.len(), to_source_span(*call_span)));
    }
    let rand_val = context.world.borrow_mut().next_u32();
    let result = (rand_val as f64) / (u32::MAX as f64);
    Ok(SpannedValue {
        value: Value::Number(result),
        span: *call_span,
    })
};
