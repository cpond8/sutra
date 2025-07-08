//! The Sutra Command-Line Interface.
//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::cli::args::{Command, SutraArgs};
use crate::cli::output::StdoutSink;
use crate::macros::{expand_macros, load_macros_from_file, MacroDef, MacroEnv, MacroRegistry};
use clap::Parser;
use std::io::Write;
use std::process;
use termcolor::WriteColor;

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
        Command::GenExpected { path } => handle_gen_expected(&path),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

use crate::ast::{Expr, Span, WithSpan};

// === General CLI Helpers (shared by multiple functions) ===

/// Converts a Path to a &str, returning an error if invalid.
fn path_to_str(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
    path.to_str().ok_or_else(|| "Invalid filename".into())
}

/// Reads a file to a String, given a path.
fn read_file_to_string(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let filename = path_to_str(path)?;
    Ok(std::fs::read_to_string(filename)?)
}

/// Parses Sutra source code into AST nodes.
fn parse_source_to_ast(source: &str) -> Result<Vec<WithSpan<Expr>>, Box<dyn std::error::Error>> {
    Ok(crate::syntax::parser::parse(source).map_err(|e| e.with_source(source))?)
}

/// Wraps AST nodes in a (do ...) if needed.
fn wrap_in_do_if_needed(ast_nodes: Vec<WithSpan<Expr>>, source: &str) -> WithSpan<Expr> {
    if ast_nodes.len() == 1 {
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
    }
}

/// Partitions AST nodes into macro definitions and user code, and builds a user macro registry.
type MacroParseResult = Result<(MacroRegistry, Vec<WithSpan<Expr>>), Box<dyn std::error::Error>>;
fn partition_and_build_user_macros(ast_nodes: Vec<WithSpan<Expr>>) -> MacroParseResult {
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(crate::is_macro_definition);
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
    Ok((user_macros, user_code))
}

/// Prints a sorted list of names with a title.
fn print_sorted_list<T: AsRef<str>>(title: &str, names: &[T]) {
    if !names.is_empty() {
        println!("\n{title}:");
        let mut sorted: Vec<_> = names.iter().map(|n| n.as_ref()).collect();
        sorted.sort();
        for name in sorted {
            println!("  {}", name);
        }
    }
}

// --- Test helpers moved to module scope ---

fn find_test_scripts(dir: &std::path::Path) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut tests = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
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

fn read_file_trimmed(path: &std::path::Path) -> std::io::Result<String> {
    Ok(std::fs::read_to_string(path)?
        .replace("\r\n", "\n")
        .trim()
        .to_string())
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;
    let program = wrap_in_do_if_needed(ast_nodes, &source);
    let mut env = build_macro_env();
    let expanded = expand_macros(program.clone(), &mut env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

fn handle_run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = read_file_to_string(path)?;
    let mut stdout_sink = StdoutSink;
    crate::run_sutra_source_with_output(&source, &mut stdout_sink)
        .map_err(|e| format!("Sutra error: {}", e))?;
    Ok(())
}

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> Result<(), Box<dyn std::error::Error>> {
    let env = build_macro_env();

    println!("Available Macros:");
    println!("================");

    let core_names: Vec<_> = env.core_macros.keys().map(|k| k.as_str()).collect();
    let user_names: Vec<_> = env.user_macros.keys().map(|k| k.as_str()).collect();

    print_sorted_list("Core Macros", &core_names);
    print_sorted_list("User-Defined Macros", &user_names);

    if core_names.is_empty() && user_names.is_empty() {
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

    let atom_names: Vec<_> = atom_registry.atoms.keys().map(|k| k.as_str()).collect();
    print_sorted_list("Atoms", &atom_names);

    if atom_names.is_empty() {
        println!("  No atoms found.");
    }

    Ok(())
}

/// Handles the `ast` subcommand.
fn handle_ast(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;

    let filename = path_to_str(path)?;
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
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;
    let (user_macros, user_code) = partition_and_build_user_macros(ast_nodes)?;

    // Use core macros from helper
    let mut env = build_macro_env();
    env.user_macros.extend(user_macros.macros);

    // Wrap user_code in a (do ...) if needed
    let program = wrap_in_do_if_needed(user_code, &source);

    // Expand macros
    let expanded =
        expand_macros(program, &mut env).map_err(|e| format!("Macro expansion error: {:?}", e))?;

    // Validation step
    let atom_registry = crate::runtime::registry::build_default_atom_registry();
    match crate::syntax::validate::validate(&expanded, &env, &atom_registry) {
        Ok(_) => {
            println!(
                "✅ Validation successful: No errors found in {}",
                path_to_str(path).unwrap_or("<unknown>")
            );
        }
        Err(e) => {
            println!(
                "❌ Validation failed in {}:",
                path_to_str(path).unwrap_or("<unknown>")
            );
            println!("   {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;
    let (user_macros, user_code) = partition_and_build_user_macros(ast_nodes)?;

    // Use core macros from helper
    let mut env = build_macro_env();
    env.user_macros.extend(user_macros.macros);

    // Wrap user_code in a (do ...) if needed
    let program = wrap_in_do_if_needed(user_code, &source);

    // Expand macros
    let expanded =
        expand_macros(program, &mut env).map_err(|e| format!("Macro expansion error: {:?}", e))?;

    // Print the expanded result
    println!("{}", expanded.value.pretty());

    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;

    // Pretty-print each top-level AST node
    for node in ast_nodes {
        println!("{}", node.value.pretty());
    }

    Ok(())
}

// --- Test outcome and helpers for test reporting ---

enum TestOutcome {
    Pass,
    Fail { expected: String, actual: String },
    Skip,
}

fn run_single_test(
    script: &std::path::Path,
    expected: &std::path::Path,
    stdout: &mut termcolor::StandardStream,
) -> Result<TestOutcome, Box<dyn std::error::Error>> {
    let script_name = script.file_name().unwrap().to_string_lossy();
    let script_src = read_file_trimmed(script)?;
    let expected_output = read_file_trimmed(expected)?;

    if script_src.contains("test/") {
        print_test_result(&script_name, TestOutcome::Skip, stdout);
        return Ok(TestOutcome::Skip);
    }

    let mut output_buf = crate::cli::output::OutputBuffer::new();
    let result = crate::run_sutra_source_with_output(&script_src, &mut output_buf);
    let actual_output = output_buf.as_str().replace("\r\n", "\n").trim().to_string();

    let pass = match result {
        Ok(_) => actual_output == expected_output,
        Err(e) => {
            let err_str = format!("{e}").replace("\r\n", "\n").trim().to_string();
            err_str == expected_output
        }
    };

    if pass {
        print_test_result(&script_name, TestOutcome::Pass, stdout);
        Ok(TestOutcome::Pass)
    } else {
        let expected_clone = expected_output.clone();
        let actual_clone = actual_output.clone();
        print_test_result(
            &script_name,
            TestOutcome::Fail {
                expected: expected_clone,
                actual: actual_clone,
            },
            stdout,
        );
        Ok(TestOutcome::Fail {
            expected: expected_output,
            actual: actual_output,
        })
    }
}

fn print_test_result(
    script_name: &str,
    outcome: TestOutcome,
    stdout: &mut termcolor::StandardStream,
) {
    use termcolor::ColorSpec;
    use TestOutcome::*;
    match outcome {
        Pass => {
            let _ = stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(termcolor::Color::Green))
                    .set_bold(true),
            );
            let _ = writeln!(stdout, "PASS: {script_name}");
        }
        Fail {
            ref expected,
            ref actual,
        } => {
            let _ = stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(termcolor::Color::Red))
                    .set_bold(true),
            );
            let _ = writeln!(stdout, "FAIL: {script_name}");
            let _ = stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(termcolor::Color::Yellow))
                    .set_bold(false),
            );
            let _ = writeln!(stdout, "  Expected: {expected:?}");
            let _ = writeln!(stdout, "  Actual:   {actual:?}");
        }
        Skip => {
            let _ = stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(termcolor::Color::Yellow))
                    .set_bold(true),
            );
            let _ = writeln!(stdout, "SKIP: {script_name} (requires test atoms)");
        }
    }
    let _ = stdout.reset();
}

/// Handles the `test` subcommand.
fn handle_test(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use termcolor::{ColorChoice, ColorSpec, StandardStream};

    let scripts = find_test_scripts(path);
    if scripts.is_empty() {
        println!("No .sutra test scripts found in {}", path.display());
        return Ok(());
    }

    let mut failed = false;
    let mut skipped = 0;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    for (script, expected) in scripts {
        match run_single_test(&script, &expected, &mut stdout)? {
            TestOutcome::Pass => {}
            TestOutcome::Fail { .. } => failed = true,
            TestOutcome::Skip => skipped += 1,
        }
    }

    if skipped > 0 {
        let _ = stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(termcolor::Color::Cyan))
                .set_bold(false),
        );
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

/// Handles the `gen-expected` subcommand.
fn handle_gen_expected(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Use the test harness from tests/common/mod.rs
    // This requires including the module at build time for CLI use
    #[path = "../../tests/common/mod.rs"]
    mod test_common;
    use test_common::{generate_expected_output, discover_test_cases, TestConfig};

    let config = TestConfig::default();
    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sutra") {
        generate_expected_output(path, &config)?;
        println!("Generated expected output for {}", path.display());
    } else if path.is_dir() {
        let test_cases = discover_test_cases(path)?;
        for case in test_cases {
            if let Err(e) = generate_expected_output(&case.sutra_file, &config) {
                eprintln!("Failed for {}: {}", case.sutra_file.display(), e);
            } else {
                println!("Generated expected output for {}", case.sutra_file.display());
            }
        }
    } else {
        eprintln!("Path must be a .sutra file or directory");
    }
    Ok(())
}

/// Builds a MacroEnv with both core and user macros loaded.
fn build_macro_env() -> MacroEnv {
    let mut core_macros = MacroRegistry::new();
    crate::macros::std::register_std_macros(&mut core_macros);

    let mut user_macros = MacroRegistry::new();
    if let Ok(macros) = load_macros_from_file("src/macros/macros.sutra") {
        for (name, template) in macros {
            user_macros
                .macros
                .insert(name, MacroDef::Template(template));
        }
    }

    MacroEnv {
        user_macros: user_macros.macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
    }
}
