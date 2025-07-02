//! Handles all user-facing output for the CLI.
//!
//! This module is responsible for pretty-printing, colorizing output,
//! formatting errors, and generating JSON. By centralizing output logic here,
//! we ensure a consistent user experience across all commands.

use crate::macros::TraceStep;
use difference::Changeset;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Prints a macro expansion trace to the console with colored diffs.
pub fn print_trace(trace: &[TraceStep]) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut last_ast_str = String::new();

    for (i, step) in trace.iter().enumerate() {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        println!("--- Step {}: {} ---", i, step.description);
        let _ = stdout.reset();

        let current_ast_str = step.ast.pretty();

        if i > 0 {
            let changeset = Changeset::new(&last_ast_str, &current_ast_str, "\n");
            for diff in changeset.diffs {
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
        } else {
            println!("{}", current_ast_str);
        }

        last_ast_str = current_ast_str;
        println!();
    }
}
