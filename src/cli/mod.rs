//! The Sutra Command-Line Interface.
//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::cli::args::{Command, SutraArgs};
use crate::macros::MacroRegistry;
use crate::{macros_std, parser};
use clap::Parser;
use std::{fs, process};

pub mod args;
pub mod output;

/// The main entry point for the CLI.
pub fn run() {
    let args = SutraArgs::parse();

    // Dispatch to the appropriate subcommand handler.
    let result = match args.command {
        Command::Macrotrace { file } => handle_macrotrace(&file),
        // Other commands will be handled here later.
        _ => {
            println!("Command not yet implemented.");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;
    let ast = parser::parse(&source).map_err(|e| e.with_source(&source))?;

    let mut registry = MacroRegistry::new();
    // TODO: This manual registration should be moved into a function in macros_std.
    registry.register("is?", macros_std::expand_is);
    registry.register("over?", macros_std::expand_over);
    registry.register("under?", macros_std::expand_under);
    registry.register("add!", macros_std::expand_add);
    registry.register("sub!", macros_std::expand_sub);
    registry.register("inc!", macros_std::expand_inc);
    registry.register("dec!", macros_std::expand_dec);

    let trace = registry.macroexpand_trace(&ast)?;
    output::print_trace(&trace);

    Ok(())
}
