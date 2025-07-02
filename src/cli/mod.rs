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

use crate::ast::{Expr, Span};

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;
    let ast_nodes = parser::parse(&source).map_err(|e| e.with_source(&source))?;

    let program = if ast_nodes.len() == 1 {
        ast_nodes.into_iter().next().unwrap()
    } else {
        let span = Span {
            start: 0,
            end: source.len(),
        };
        Expr::List(
            {
                let mut vec = vec![Expr::Symbol("do".to_string(), span.clone())];
                vec.extend(ast_nodes);
                vec
            },
            span,
        )
    };

    let mut registry = MacroRegistry::new();
    // Centralized registration of all standard macros.
    macros_std::register_std_macros(&mut registry);

    let trace = registry.macroexpand_trace(&program)?;
    output::print_trace(&trace);

    Ok(())
}
