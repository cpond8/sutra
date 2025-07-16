//! # Test and Expectation Parsing
//!
//! This module provides pure, stateless parsing utilities for extracting test cases and expectations
//! from Sutra source files. It is responsible for parsing `(test ...)` and `(expect ...)` forms into
//! structured Rust data types.
//!
//! ## Philosophy Alignment
//! - **Minimalism:** Only parsing, no execution or side effects.
//! - **Compositionality:** All parsers are pure functions.
//! - **Transparency:** All parse errors are surfaced as miette diagnostics.
//! - **Extensibility:** New forms are added by extending the parser.

/// Parse a `.sutra` file into a list of test cases.
pub fn parse_tests(/* TODO: source or AST type */) -> Vec</* TODO: TestCase type */> {
    // TODO: Implement parser
    vec![]
}