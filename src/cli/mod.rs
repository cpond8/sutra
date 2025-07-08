//! The Sutra Command-Line Interface.
//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::cli::args::{Command, SutraArgs};
use crate::cli::output::StdoutSink;
use crate::macros::{expand_macros, load_macros_from_file, MacroDef, MacroEnv, MacroRegistry};
use crate::syntax::error::io_error;
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
        Command::Run { file } => handle_run(&file),
        Command::ListMacros => handle_list_macros(),
        Command::ListAtoms => handle_list_atoms(),
        Command::Ast { file } => handle_ast(&file),
        Command::Validate { file } => handle_validate(&file),
        Command::Macroexpand { file } => handle_macroexpand(&file),
        Command::Format { file } => handle_format(&file),
        Command::Test { path } => handle_test(&path),
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
    let ast_nodes = crate::syntax::parser::parse(&source).map_err(|e| e.with_source(&source))?;

    let program: WithSpan<Expr> = if ast_nodes.len() == 1 {
        ast_nodes
            .into_iter()
            .next()
            .ok_or_else(|| io_error("No AST nodes found", None))?
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

    // Build core macro registry
    let mut core_macros = MacroRegistry::new();
    crate::macros::std::register_std_macros(&mut core_macros);

    // Load user-defined/stdlib macros from src/macros/macros.sutra
    let mut user_macros = MacroRegistry::new();
    match load_macros_from_file("src/macros/macros.sutra") {
        Ok(macros) => {
            for (name, template) in macros {
                user_macros
                    .macros
                    .insert(name, MacroDef::Template(template));
            }
        }
        Err(e) => {
            eprintln!("Error loading macros from src/macros/macros.sutra: {}", e);
            std::process::exit(1);
        }
    }

    let mut env = MacroEnv {
        user_macros: user_macros.macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
    };
    let expanded = expand_macros(program.clone(), &mut env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
    println!("{}", expanded.value.pretty());
    // Optionally print trace here if desired
    Ok(())
}

fn handle_run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = std::fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read '{}': {}", filename, e))?;
    let mut stdout_sink = StdoutSink;
    crate::run_sutra_source_with_output(&source, &mut stdout_sink)
        .map_err(|e| format!("Sutra error: {}", e))?;
    Ok(())
}

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> Result<(), Box<dyn std::error::Error>> {
    // Build core macro registry
    let mut core_macros = MacroRegistry::new();
    crate::macros::std::register_std_macros(&mut core_macros);

    // Load user-defined/stdlib macros from src/macros/macros.sutra
    let mut user_macros = MacroRegistry::new();
    match load_macros_from_file("src/macros/macros.sutra") {
        Ok(macros) => {
            for (name, template) in macros {
                user_macros
                    .macros
                    .insert(name, MacroDef::Template(template));
            }
        }
        Err(_) => {
            // Ignore error - file might not exist
        }
    }

    println!("Available Macros:");
    println!("================");

    if !core_macros.macros.is_empty() {
        println!("\nCore Macros:");
        let mut core_names: Vec<_> = core_macros.macros.keys().collect();
        core_names.sort();
        for name in core_names {
            println!("  {}", name);
        }
    }

    if !user_macros.macros.is_empty() {
        println!("\nUser-Defined Macros:");
        let mut user_names: Vec<_> = user_macros.macros.keys().collect();
        user_names.sort();
        for name in user_names {
            println!("  {}", name);
        }
    }

    if core_macros.macros.is_empty() && user_macros.macros.is_empty() {
        println!("  No macros found.");
    }

    Ok(())
}

/// Handles the `list-atoms` subcommand.
fn handle_list_atoms() -> Result<(), Box<dyn std::error::Error>> {
    use crate::runtime::registry::build_default_atom_registry;

    let atom_registry = build_default_atom_registry();

    println!("Available Atoms:");
    println!("===============");

    if !atom_registry.atoms.is_empty() {
        let mut atom_names: Vec<_> = atom_registry.atoms.keys().collect();
        atom_names.sort();
        for name in atom_names {
            println!("  {}", name);
        }
    } else {
        println!("  No atoms found.");
    }

    Ok(())
}

/// Handles the `ast` subcommand.
fn handle_ast(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;
    let ast_nodes = crate::syntax::parser::parse(&source).map_err(|e| e.with_source(&source))?;

    println!("AST for {}:", filename);
    println!("={}=", "=".repeat(filename.len() + 9));

    if ast_nodes.is_empty() {
        println!("(empty)");
    } else {
        for (i, node) in ast_nodes.iter().enumerate() {
            if ast_nodes.len() > 1 {
                println!("\nNode {}:", i + 1);
            }
            println!("{:#?}", node);
        }
    }

    Ok(())
}

/// Handles the `validate` subcommand.
fn handle_validate(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;

    // Parse the source into AST nodes
    let ast_nodes = crate::syntax::parser::parse(&source).map_err(|e| e.with_source(&source))?;

    // Partition AST nodes: macro definitions vs user code
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(crate::is_macro_definition);

    // Build macro registry from macro_defs
    let mut user_macros = MacroRegistry::new();
    for macro_expr in macro_defs {
        let (name, template) = crate::parse_macro_definition(&macro_expr)?;
        if user_macros.macros.contains_key(&name) {
            return Err(format!("Duplicate macro name '{}'.", name).into());
        }
        user_macros
            .macros
            .insert(name, MacroDef::Template(template));
    }

    // Build core macro registry (standard macros)
    let mut core_macros = MacroRegistry::new();
    crate::macros::std::register_std_macros(&mut core_macros);

    // Build MacroEnv
    let mut env = MacroEnv {
        user_macros: user_macros.macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
    };

    // Wrap user_code in a (do ...) if needed
    let program = crate::wrap_in_do(user_code);

    // Expand macros
    let expanded =
        expand_macros(program, &mut env).map_err(|e| format!("Macro expansion error: {:?}", e))?;

    // Validation step
    let atom_registry = crate::runtime::registry::build_default_atom_registry();
    match crate::syntax::validate::validate(&expanded, &env, &atom_registry) {
        Ok(_) => {
            println!("✅ Validation successful: No errors found in {}", filename);
        }
        Err(e) => {
            println!("❌ Validation failed in {}:", filename);
            println!("   {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;

    // Parse the source into AST nodes
    let ast_nodes = crate::syntax::parser::parse(&source).map_err(|e| e.with_source(&source))?;

    // Partition AST nodes: macro definitions vs user code
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(crate::is_macro_definition);

    // Build macro registry from macro_defs
    let mut user_macros = MacroRegistry::new();
    for macro_expr in macro_defs {
        let (name, template) = crate::parse_macro_definition(&macro_expr)?;
        if user_macros.macros.contains_key(&name) {
            return Err(format!("Duplicate macro name '{}'.", name).into());
        }
        user_macros
            .macros
            .insert(name, MacroDef::Template(template));
    }

    // Build core macro registry
    let mut core_macros = MacroRegistry::new();
    crate::macros::std::register_std_macros(&mut core_macros);

    // Build MacroEnv
    let mut env = MacroEnv {
        user_macros: user_macros.macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
    };

    // Wrap user_code in a (do ...) if needed
    let program = crate::wrap_in_do(user_code);

    // Expand macros
    let expanded =
        expand_macros(program, &mut env).map_err(|e| format!("Macro expansion error: {:?}", e))?;

    // Print the expanded result
    println!("{}", expanded.value.pretty());

    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.to_str().ok_or("Invalid filename")?;
    let source = fs::read_to_string(filename)?;
    let ast_nodes = crate::syntax::parser::parse(&source).map_err(|e| e.with_source(&source))?;

    // Pretty-print each top-level AST node
    for node in ast_nodes {
        println!("{}", node.value.pretty());
    }

    Ok(())
}

/// Handles the `test` subcommand.
fn handle_test(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Write};
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    use walkdir::WalkDir;

    // Function to find test scripts in a directory
    fn find_test_scripts(dir: &std::path::Path) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
        let mut tests = Vec::new();
        for entry in WalkDir::new(dir) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "sutra").unwrap_or(false) {
                let expected = path.with_extension("expected");
                if expected.exists() {
                    tests.push((path.to_path_buf(), expected));
                }
            }
        }
        tests
    }

    // Function to read file with consistent line ending handling
    fn read_file_trimmed(path: &std::path::Path) -> io::Result<String> {
        Ok(fs::read_to_string(path)?
            .replace("\r\n", "\n")
            .trim()
            .to_string())
    }

    let scripts = find_test_scripts(path);
    if scripts.is_empty() {
        println!("No .sutra test scripts found in {}", path.display());
        return Ok(());
    }

    let mut failed = false;
    let mut skipped = 0;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    for (script, expected) in scripts {
        let script_name = script.file_name().unwrap().to_string_lossy();
        let script_src = read_file_trimmed(&script)?;
        let expected_output = read_file_trimmed(&expected)?;

        // Check if this is a test script that requires special test atoms
        let uses_test_atoms = script_src.contains("test/");

        if uses_test_atoms {
            // Skip test scripts that use test atoms when not in test mode
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
            let _ = writeln!(stdout, "SKIP: {script_name} (requires test atoms)");
            let _ = stdout.reset();
            skipped += 1;
            continue;
        }

        // Run the script using the Sutra engine public API with OutputBuffer
        let mut output_buf = crate::cli::output::OutputBuffer::new();
        let result = crate::run_sutra_source_with_output(&script_src, &mut output_buf);
        let actual_output = output_buf.as_str().replace("\r\n", "\n").trim().to_string();

        let pass = match result {
            Ok(_) => actual_output == expected_output,
            Err(e) => {
                // If expected output is an error message, compare to error string
                let err_str = format!("{e}").replace("\r\n", "\n").trim().to_string();
                err_str == expected_output
            }
        };

        if pass {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
            let _ = writeln!(stdout, "PASS: {script_name}");
        } else {
            failed = true;
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
            let _ = writeln!(stdout, "FAIL: {script_name}");
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(false));
            let _ = writeln!(stdout, "  Expected: {expected_output:?}");
            let _ = writeln!(stdout, "  Actual:   {actual_output:?}");
        }
        let _ = stdout.reset();
    }

    // Print summary
    let _ = stdout.reset();
    if skipped > 0 {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(false));
        let _ = writeln!(
            stdout,
            "\nNote: {skipped} test(s) skipped (use 'cargo test' to run all tests)"
        );
        let _ = stdout.reset();
    }

    if failed {
        return Err("One or more test scripts failed".into());
    }

    Ok(())
}
