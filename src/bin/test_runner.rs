// Sutra idiomatic test runner: uses the shared test_harness module
// Usage: cargo run --bin test_runner [category] [suite]

use std::env;
use sutra::test_harness;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let results = test_harness::run_tests_with_args(&args);
    if results.failed > 0 {
        std::process::exit(1);
    }
}
