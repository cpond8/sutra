//! Standalone test harness binary for advanced workflows.

use clap::{Arg, Command};
use std::env;
use std::process;

mod common;
use common::harness;

fn main() {
    let matches = Command::new("sutra-harness")
        .about("Standalone test harness for Sutra with advanced features")
        .arg(
            Arg::new("filter")
                .short('f')
                .long("filter")
                .num_args(1)
                .help("Run only tests matching this pattern"),
        )
        .arg(
            Arg::new("update_snapshots")
                .short('u')
                .long("update-snapshots")
                .num_args(0)
                .help("Update diagnostic snapshots on mismatch"),
        )
        .get_matches();

    // Set environment variables for integration with shared config
    if matches.contains_id("update_snapshots") {
        env::set_var("UPDATE_SNAPSHOTS", "1");
    }
    if let Some(filter) = matches.get_one::<String>("filter") {
        env::set_var("TEST_FILTER", filter);
    }

    let filter = matches.get_one::<String>("filter").map(|s| s.as_str());
    let (passed, failed, skipped) = harness::run_default_tests(filter);

    println!(
        "\nSummary: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    if failed > 0 {
        process::exit(1);
    }
}
