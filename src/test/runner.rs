use std::{cell::RefCell, rc::Rc};

use crate::{
    atoms::{build_canonical_macro_env, build_canonical_world, SharedOutput},
    discovery::ASTDefinition,
    engine::{evaluate, EngineOutputBuffer, ExecutionPipeline, MacroProcessor},
    errors::{to_source_span, ErrorCategory, ErrorKind, ErrorReporting, SourceContext, SutraError},
    prelude::*,
    syntax::parser,
    validation::ValidationContext,
};

/// Executes test code with proper macro expansion and special form preservation.
pub struct TestRunner;

impl TestRunner {
    pub fn execute_test(
        test_body: &[AstNode],
        output: SharedOutput,
        test_file: Option<String>,
        test_name: Option<String>,
        source_file: SourceContext,
    ) -> Result<(), SutraError> {
        // Use a macro processor to handle expansion and evaluation correctly
        let processor = MacroProcessor::default().with_test_context(
            test_file.clone().unwrap_or_default(),
            test_name.clone().unwrap_or_default(),
        );
        let world = build_canonical_world();
        let mut macro_env = build_canonical_macro_env().expect("Standard macro env should build");

        // Try macro expansion
        let expanded_node =
            processor.process_with_existing_macros(test_body.to_vec(), &mut macro_env)?;

        // Use the actual test file source for error reporting
        let source_context = source_file;

        // Validate the expanded AST
        processor.validate_expanded_ast(
            &expanded_node,
            &macro_env,
            &world.borrow(),
            &source_context,
        )?;

        // Use the builder for evaluation context
        let result = evaluate(&expanded_node, world, output.clone(), source_context)?;

        // If the result is not nil, emit it to the output buffer
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    pub fn execute_ast(
        nodes: &[AstNode],
        source_context: &SourceContext,
    ) -> Result<Value, SutraError> {
        let pipeline = ExecutionPipeline::default();
        let output = SharedOutput::new(crate::engine::EngineOutputBuffer::new());
        pipeline.execute_nodes(nodes, output, source_context.clone())
    }

    pub fn run_single_test(test_form: &ASTDefinition) -> Result<(), SutraError> {
        let source_context = &test_form.source_file;
        let expected = Self::extract_expectation(test_form, source_context)?;

        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };

        // Guard: parse error expectation with single string body
        if matches!(&expected, Expectation::Error(ErrorCategory::Parse))
            && test_form.body.len() == 1
            && matches!(*test_form.body[0].value, Expr::String(_, _))
        {
            let Expr::String(ref code, _) = *test_form.body[0].value else {
                unreachable!()
            };
            let parse_result = parser::parse(code, source_context.clone());
            return match parse_result {
                Err(e) if e.kind.category() == ErrorCategory::Parse => Ok(()),
                Err(e) => Err(context.report(
                    ErrorKind::AssertionFailure {
                        message: format!(
                            "Expected parse error, but got different error: {:?}",
                            e.kind.category()
                        ),
                        test_name: test_form.name.clone(),
                    },
                    to_source_span(test_form.span),
                )),
                Ok(_) => Err(context.report(
                    ErrorKind::AssertionFailure {
                        message: "Expected parse error, but parsing succeeded".to_string(),
                        test_name: test_form.name.clone(),
                    },
                    to_source_span(test_form.span),
                )),
            };
        }

        // Guard: output expectation
        if let Expectation::Output(ref expected_output) = expected {
            let output_buffer = Rc::new(RefCell::new(EngineOutputBuffer::new()));
            let shared_output = SharedOutput(output_buffer.clone());
            let result = Self::execute_test(
                &test_form.body,
                shared_output,
                Some(test_form.source_file.name.clone()),
                Some(test_form.name.clone()),
                test_form.source_file.clone(),
            );
            let actual_output_owned = output_buffer.borrow().as_str().to_owned();
            if actual_output_owned != *expected_output {
                return Err(context.report(
                    ErrorKind::AssertionFailure {
                        message: format!(
                            "Expected output {:?}, got {:?}",
                            expected_output, actual_output_owned
                        ),
                        test_name: test_form.name.clone(),
                    },
                    to_source_span(test_form.span),
                ));
            }
            if let Err(e) = result {
                return Err(e);
            }
            return Ok(());
        }

        // All other tests
        match Self::execute_ast(&test_form.body, &test_form.source_file) {
            Ok(actual) => Self::check_success_test(test_form, &expected, actual, &source_context),
            Err(e) => Self::check_error_test(test_form, &expected, e, &source_context),
        }
    }

    fn check_success_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual: Value,
        source_context: &SourceContext,
    ) -> Result<(), SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };
        match expected {
            // Success case: actual matches expected value
            Expectation::Value(expected_value) if actual == *expected_value => Ok(()),
            // Failure case: actual doesn't match expected value
            Expectation::Value(expected_value) => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: format!("Expected {}, got {}", expected_value, actual),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
            // Failure case: expected error but got success
            Expectation::Error(_) => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "Expected error, but test succeeded".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
            // Failure case: output expectation in value test path
            Expectation::Output(_) => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "Output expectation not handled in value test path. Output expectations should be handled in the output test path".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
        }
    }

    fn check_error_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual_error: SutraError,
        source_context: &SourceContext,
    ) -> Result<(), SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };
        match expected {
            // Success case: error type matches expected
            Expectation::Error(expected_category)
                if actual_error.kind.category() == *expected_category =>
            {
                Ok(())
            }
            // Failure case: expected error but got different type
            Expectation::Error(expected_category) => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: format!(
                        "Expected error category {:?}, got {:?}",
                        expected_category,
                        actual_error.kind.category()
                    ),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
            // Failure case: expected success but got error
            Expectation::Value(_) => Err(actual_error),
            // Failure case: output expectation in error test path
            Expectation::Output(_) => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "Output expectation not handled in error test path. Output expectations should be handled in the output test path".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_expectation(
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<Expectation, SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };

        // Get expect form from test definition
        let expect = test_form.expect_form.as_ref().ok_or_else(|| {
            context.report(
                ErrorKind::AssertionFailure {
                    message: "Test is missing expect form".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )
        })?;

        // Validate expect form is a list
        let Expr::List(items, _) = &*expect.value else {
            return Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "expect form must be a list".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            ));
        };

        // Look for value or error clause in expect form
        for item in items {
            if let Some(expectation) = Self::extract_clause(&item, test_form, source_context)? {
                return Ok(expectation);
            }
        }

        Err(context.report(
            ErrorKind::AssertionFailure {
                message: "missing (value <expected>) or (error <type>) in expect form".to_string(),
                test_name: test_form.name.clone(),
            },
            to_source_span(test_form.span),
        ))
    }

    fn extract_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<Option<Expectation>, SutraError> {
        let Expr::List(items, _) = &*item.value else {
            return Ok(None);
        };
        if items.is_empty() {
            return Ok(None);
        }
        let Expr::Symbol(keyword, _) = &*items[0].value else {
            return Ok(None);
        };
        if keyword == "value" {
            // Accept (value x y z) as a list value if more than two elements
            if items.len() == 2 {
                let v = Self::extract_value(&items[1], test_form, source_context)?;
                return Ok(Some(Expectation::Value(v)));
            } else if items.len() > 2 {
                // Build a list value from all elements after the keyword
                let mut result = Value::Nil;
                for item in items[1..].iter().rev() {
                    let cell = crate::ast::value::ConsCell {
                        car: Self::extract_value(item, test_form, source_context)?,
                        cdr: result,
                    };
                    result = Value::Cons(Rc::new(cell));
                }
                return Ok(Some(Expectation::Value(result)));
            }
        }
        if keyword == "error" && items.len() == 2 {
            let e = Self::extract_error_type(&items[1], test_form, source_context)?;
            return Ok(Some(Expectation::Error(e)));
        }
        if keyword == "output" && items.len() == 2 {
            let o = Self::extract_output(&items[1], test_form, source_context)?;
            return Ok(Some(Expectation::Output(o)));
        }
        Ok(None)
    }

    fn extract_value(
        value_node: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<Value, SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };
        // Convert AST node to Value based on type
        match &*value_node.value {
            Expr::Number(n, _) => Ok(Value::Number(*n)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::Symbol(s, _) if s == "nil" => Ok(Value::Nil),
            Expr::Symbol(s, _) => Ok(Value::Symbol(s.clone())),
            Expr::Quote(inner, _) => Ok(Value::Quote(Box::new(Self::extract_value(
                inner,
                test_form,
                source_context,
            )?))),
            Expr::List(items, _) => {
                let mut result = Value::Nil;
                for item in items.iter().rev() {
                    let cell = crate::ast::value::ConsCell {
                        car: Self::extract_value(item, test_form, source_context)?,
                        cdr: result,
                    };
                    result = Value::Cons(Rc::new(cell));
                }
                Ok(result)
            }
            _ => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "unsupported expected value type".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_error_type(
        error_node: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<ErrorCategory, SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };
        // Extract error type symbol
        let Expr::Symbol(error_type, _) = &*error_node.value else {
            return Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "error type must be a symbol".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            ));
        };

        // Map symbol to ErrorCategory enum
        match error_type.as_str() {
            "Parse" => Ok(ErrorCategory::Parse),
            "Validation" => Ok(ErrorCategory::Validation),
            "Runtime" => Ok(ErrorCategory::Runtime),
            "Test" => Ok(ErrorCategory::Test),
            _ => Err(context.report(
                ErrorKind::AssertionFailure {
                    message: format!("unknown error category: {}", error_type),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_output(
        output_node: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<String, SutraError> {
        let context = ValidationContext {
            source: source_context.clone(),
            phase: "testing".to_string(),
        };
        let Expr::String(s, _) = &*output_node.value else {
            return Err(context.report(
                ErrorKind::AssertionFailure {
                    message: "output expectation must be a string".to_string(),
                    test_name: test_form.name.clone(),
                },
                to_source_span(test_form.span),
            ));
        };
        Ok(s.clone())
    }
}

/// Represents expected test outcome (success value or error type)
#[derive(Debug, Clone, PartialEq)]
enum Expectation {
    Value(Value),
    Error(ErrorCategory),
    Output(String),
}
