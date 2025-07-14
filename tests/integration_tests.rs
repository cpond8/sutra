//! Integration test entry point for cargo test compatibility.

mod common;
use common::harness;
use std::env;

#[test]
fn run_all_sutra_tests() {
    let filter = env::var("TEST_FILTER").ok();
    let (passed, failed, skipped) = harness::run_default_tests(filter.as_deref());

    println!(
        "Integration test summary: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    if failed > 0 {
        panic!("{} test(s) failed in integration mode", failed);
    }
}
