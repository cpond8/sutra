//! The Sutra Command-Line Interface.
//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::cli::args::{Command, SutraArgs};
use crate::macros::{
    load_macros_from_file, MacroDef, MacroExpander, MacroRegistry, SutraMacroContext,
    SutraMacroExpander,
};
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

use crate::ast::{Expr, Span, WithSpan};

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;
    let ast_nodes = parser::parse(&source).map_err(|e| e.with_source(&source))?;

    let program: WithSpan<Expr> = if ast_nodes.len() == 1 {
        ast_nodes.into_iter().next().unwrap()
    } else {
        let span = Span {
            start: 0,
            end: source.len(),
        };
        let do_symbol = WithSpan {
            value: Expr::Symbol("do".to_string(), span.clone()),
            span: span.clone(),
        };
        let mut items = Vec::with_capacity(ast_nodes.len() + 1);
        items.push(do_symbol);
        items.extend(ast_nodes);
        WithSpan {
            value: Expr::List(items, span.clone()),
            span,
        }
    };

    let mut registry = MacroRegistry::new();
    // Centralized registration of all standard macros.
    macros_std::register_std_macros(&mut registry);

    // Load user-defined/stdlib macros from macros.sutra
    match load_macros_from_file("macros.sutra") {
        Ok(macros) => {
            for (name, template) in macros {
                registry.macros.insert(name, MacroDef::Template(template));
            }
        }
        Err(e) => {
            eprintln!("Error loading macros from macros.sutra: {}", e);
            std::process::exit(1);
        }
    }

    let context = SutraMacroContext {
        registry,
        hygiene_scope: None,
    };
    let expander = MacroExpander::default();
    let expanded = expander
        .expand_macros(program.clone(), &context)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
    println!("{}", expanded.value.pretty());

    Ok(())
}
