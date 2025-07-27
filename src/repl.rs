//! Sutra REPL (Read-Eval-Print Loop)
//!
//! Provides an interactive shell for evaluating Sutra expressions with persistent state.

use std::io::{self, Write};

use crate::{atoms::SharedOutput, cli::ExecutionPipeline, errors::print_error, EngineStdoutSink};

/// REPL state that persists across evaluations
pub struct ReplState {
    pipeline: ExecutionPipeline,
    line_number: usize,
}

impl ReplState {
    pub fn new() -> Self {
        Self {
            pipeline: ExecutionPipeline::default(),
            line_number: 1,
        }
    }

    /// Evaluate a line of Sutra code in the persistent REPL context
    pub fn eval_line(&mut self, input: &str) -> Result<(), ()> {
        let source_name = format!("<repl:{}>", self.line_number);
        let output = SharedOutput::new(EngineStdoutSink);

        match self.pipeline.execute(input, output, &source_name) {
            Ok(()) => {
                self.line_number += 1;
                Ok(())
            }
            Err(e) => {
                print_error(e);
                self.line_number += 1;
                Err(())
            }
        }
    }
}

/// Main REPL entry point
pub fn run_repl() {
    println!("Sutra REPL v0.1.0");
    println!("Type :help for help, :quit to exit, :clear to reset the state");
    println!();

    let mut repl_state = ReplState::new();
    let mut input_buffer = String::new();

    loop {
        // Print prompt
        if input_buffer.is_empty() {
            print!("sutra> ");
        } else {
            print!("    -> ");
        }
        io::stdout().flush().unwrap();

        // Read input
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!("\nGoodbye!");
                break;
            }
            Ok(_) => {
                let line = line.trim();

                // Handle special commands
                if input_buffer.is_empty() && line.starts_with(':') {
                    match handle_repl_command(line, &mut repl_state) {
                        ReplCommand::Continue => continue,
                        ReplCommand::Quit => break,
                    }
                }

                // Accumulate input
                if !input_buffer.is_empty() {
                    input_buffer.push(' ');
                }
                input_buffer.push_str(line);

                // Check if we have a complete expression
                if is_complete_expression(&input_buffer) {
                    // Evaluate the complete expression
                    let _ = repl_state.eval_line(&input_buffer);
                    input_buffer.clear();
                } else if line.is_empty() {
                    // Empty line with incomplete expression - try to evaluate anyway
                    let _ = repl_state.eval_line(&input_buffer);
                    input_buffer.clear();
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

/// REPL command results
enum ReplCommand {
    Continue,
    Quit,
}

/// Handle special REPL commands that start with ':'
fn handle_repl_command(command: &str, state: &mut ReplState) -> ReplCommand {
    match command.to_ascii_lowercase().as_str() {
        ":help" | ":h" => {
            println!("Sutra REPL Commands:");
            println!("  :help, :h     Show this help");
            println!("  :quit, :q     Exit the REPL");
            println!("  :clear, :c    Clear context and reset state");
            println!();
            println!("Enter Sutra expressions to evaluate them.");
            println!("Multi-line expressions are supported.");
            ReplCommand::Continue
        }
        ":quit" | ":q" => {
            println!("Goodbye!");
            ReplCommand::Quit
        }
        ":clear" | ":c" => {
            // Clear screen using ANSI escape codes
            print!("\x1B[2J\x1B[1;1H");
            io::stdout().flush().unwrap();

            // Reset pipeline to clear all user-defined variables and functions
            state.pipeline = ExecutionPipeline::default();
            println!("Context cleared.");
            ReplCommand::Continue
        }
        _ => {
            println!(
                "Unknown command: {}. Type :help for available commands.",
                command
            );
            ReplCommand::Continue
        }
    }
}

/// Simple heuristic to check if an expression is complete
/// This is a basic implementation - a more sophisticated version would parse the AST
fn is_complete_expression(input: &str) -> bool {
    let trimmed = input.trim();

    // Empty input is not complete
    if trimmed.is_empty() {
        return false;
    }

    // Count parentheses for balance
    let mut paren_count = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in trimmed.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '(' if !in_string => paren_count += 1,
            ')' if !in_string => paren_count -= 1,
            _ => {}
        }
    }

    // Expression is complete if parentheses are balanced and we're not in a string
    paren_count == 0 && !in_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_complete_expression() {
        assert!(is_complete_expression("42"));
        assert!(is_complete_expression("(+ 1 2)"));
        assert!(is_complete_expression("(do (define x 10) (+ x 5))"));
        assert!(is_complete_expression("\"hello world\""));

        assert!(!is_complete_expression("(+ 1"));
        assert!(!is_complete_expression("(do (define x 10)"));
        assert!(!is_complete_expression("\"unclosed string"));
        assert!(!is_complete_expression(""));
    }
}
