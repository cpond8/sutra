use crate::engine::EngineOutputBuffer;
use crate::prelude::*;
use crate::syntax::parser::to_source_span;
use crate::{discovery::ASTDefinition, engine::ExecutionPipeline};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes test code with proper macro expansion and special form preservation.
pub struct TestRunner;

impl TestRunner {
    pub fn execute_test(test_body: &AstNode, output: SharedOutput) -> Result<(), SutraError> {
        let pipeline = ExecutionPipeline::default();
        let result = pipeline.execute_nodes(&[test_body.clone()])?;

        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    pub fn execute_ast(nodes: &[AstNode]) -> Result<Value, SutraError> {
        let pipeline = ExecutionPipeline::default();
        pipeline.execute_nodes(nodes)
    }

    pub fn run_single_test(test_form: &ASTDefinition) -> Result<(), SutraError> {
        // Extract expected result from test form
        let expected = Self::extract_expectation(test_form)?;

        // Execute test body and check result
        if let Expectation::Output(ref expected_output) = expected {
            // Output expectation: capture output and compare
            let output_buffer = Rc::new(RefCell::new(EngineOutputBuffer::new()));
            let shared_output = crate::atoms::SharedOutput(output_buffer.clone());
            let result = Self::execute_test(&test_form.body[0], shared_output);
            let actual_output_owned = {
                let buf_ref = output_buffer.borrow();
                buf_ref.as_str().to_owned()
            };
            if actual_output_owned != *expected_output {
                return Err(SutraError::TestAssertion {
                    message: format!(
                        "Expected output {:?}, got {:?}",
                        expected_output, actual_output_owned
                    ),
                    src: test_form.source_file.as_ref().clone(),
                    span: to_source_span(test_form.span),
                    expected: Some(expected_output.clone()),
                    actual: Some(actual_output_owned),
                });
            }
            if let Err(e) = result {
                return Err(e);
            }
            return Ok(());
        }
        // Value and error expectations (existing logic)
        match Self::execute_ast(&test_form.body) {
            Ok(actual) => Self::check_success_test(test_form, &expected, actual),
            Err(e) => Self::check_error_test(test_form, &expected, e),
        }
    }

    fn check_success_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual: Value,
    ) -> Result<(), SutraError> {
        match expected {
            // Success case: actual matches expected value
            Expectation::Value(expected_value) if actual == *expected_value => Ok(()),
            // Failure case: actual doesn't match expected value
            Expectation::Value(expected_value) => Err(SutraError::TestAssertion {
                message: format!(
                    "\"{}\"\nExpected {}, got {}",
                    test_form.name, expected_value, actual
                ),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                expected: Some(expected_value.to_string()),
                actual: Some(actual.to_string()),
            }),
            // Failure case: expected error but got success
            Expectation::Error(_) => Err(SutraError::TestStructure {
                issue: "Expected error, but test succeeded".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            }),
            // Failure case: output expectation in value test path
            Expectation::Output(_) => Err(SutraError::TestStructure {
                issue: "Output expectation not handled in value test path".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: Some(
                    "Output expectations should be handled in the output test path".to_string(),
                ),
            }),
        }
    }

    fn check_error_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual_error: SutraError,
    ) -> Result<(), SutraError> {
        match expected {
            // Success case: error type matches expected
            Expectation::Error(expected_type) if actual_error.error_type() == *expected_type => {
                Ok(())
            }
            // Failure case: expected error but got different type
            Expectation::Error(expected_type) => Err(SutraError::TestStructure {
                issue: format!(
                    "Expected error type {}, got {}",
                    expected_type,
                    actual_error.error_type()
                ),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            }),
            // Failure case: expected success but got error
            Expectation::Value(_) => Err(actual_error),
            // Failure case: output expectation in error test path
            Expectation::Output(_) => Err(SutraError::TestStructure {
                issue: "Output expectation not handled in error test path".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: Some(
                    "Output expectations should be handled in the output test path".to_string(),
                ),
            }),
        }
    }

    fn extract_expectation(test_form: &ASTDefinition) -> Result<Expectation, SutraError> {
        // Get expect form from test definition
        let expect = test_form
            .expect_form
            .as_ref()
            .ok_or_else(|| SutraError::TestStructure {
                issue: "Test is missing expect form".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            })?;

        // Validate expect form is a list
        let Expr::List(items, _) = &*expect.value else {
            return Err(SutraError::TestStructure {
                issue: "expect form must be a list".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            });
        };

        // Look for value or error clause in expect form
        for item in items {
            if let Some(expectation) = Self::extract_clause(item, test_form)? {
                return Ok(expectation);
            }
        }

        Err(SutraError::TestStructure {
            issue: "missing (value <expected>) or (error <type>) in expect form".to_string(),
            test_name: test_form.name.clone(),
            src: test_form.source_file.as_ref().clone(),
            span: to_source_span(test_form.span),
            suggestion: None,
        })
    }

    fn extract_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<Option<Expectation>, SutraError> {
        let Expr::List(items, _) = &*item.value else {
            return Ok(None);
        };
        if items.len() != 2 {
            return Ok(None);
        }
        let Expr::Symbol(keyword, _) = &*items[0].value else {
            return Ok(None);
        };
        if keyword == "value" {
            let v = Self::extract_value(&items[1], test_form)?;
            return Ok(Some(Expectation::Value(v)));
        }
        if keyword == "error" {
            let e = Self::extract_error_type(&items[1], test_form)?;
            return Ok(Some(Expectation::Error(e)));
        }
        if keyword == "output" {
            let o = Self::extract_output(&items[1], test_form)?;
            return Ok(Some(Expectation::Output(o)));
        }
        Ok(None)
    }

    fn extract_value(value_node: &AstNode, test_form: &ASTDefinition) -> Result<Value, SutraError> {
        // Convert AST node to Value based on type
        match &*value_node.value {
            Expr::Number(n, _) => Ok(Value::Number(*n)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::Symbol(s, _) if s == "nil" => Ok(Value::Nil),
            _ => Err(SutraError::TestStructure {
                issue: "unsupported expected value type".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            }),
        }
    }

    fn extract_error_type(
        error_node: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<ErrorType, SutraError> {
        // Extract error type symbol
        let Expr::Symbol(error_type, _) = &*error_node.value else {
            return Err(SutraError::TestStructure {
                issue: "error type must be a symbol".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            });
        };

        // Map symbol to ErrorType enum
        match error_type.as_str() {
            "Parse" => Ok(ErrorType::Parse),
            "Validation" => Ok(ErrorType::Validation),
            "Eval" => Ok(ErrorType::Eval),
            "TypeError" => Ok(ErrorType::TypeError),
            "Internal" => Ok(ErrorType::Internal),
            "TestFailure" => Ok(ErrorType::TestFailure),
            _ => Err(SutraError::TestStructure {
                issue: format!("unknown error type: {}", error_type),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            }),
        }
    }

    fn extract_output(
        output_node: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<String, SutraError> {
        let Expr::String(s, _) = &*output_node.value else {
            return Err(SutraError::TestStructure {
                issue: "output expectation must be a string".to_string(),
                test_name: test_form.name.clone(),
                src: test_form.source_file.as_ref().clone(),
                span: to_source_span(test_form.span),
                suggestion: None,
            });
        };
        Ok(s.clone())
    }
}

/// Represents expected test outcome (success value or error type)
#[derive(Debug, Clone, PartialEq)]
enum Expectation {
    Value(Value),
    Error(ErrorType),
    Output(String),
}
