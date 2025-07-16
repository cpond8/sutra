//! # Sutra Test Harness (Root)
//!
//! This module orchestrates the compositional, miette-powered test harness for Sutra.
//!
//! ## Philosophy Alignment
//! - **Minimalism:** Each submodule has a single responsibility.
//! - **Compositionality:** All logic is pure and reusable.
//! - **Transparency:** All diagnostics and results are surfaced via miette.
//! - **Extensibility:** New tags, pipeline stages, and features are easy to add.
//!
//! See submodules for details.

pub mod parse;
pub mod expectation;
pub mod runner;
pub mod reporting;
pub mod isolation;
pub mod legacy;
pub mod util;
pub mod snapshot;