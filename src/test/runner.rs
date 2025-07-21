use crate::prelude::*;
use crate::{discovery::ASTDefinition, engine::ExecutionPipeline};

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
            Expectation::Value(expected_value) => Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nExpected {}, got {}", test_form.name, expected_value, actual),
                &test_form.source_file,
                test_form.span
            )),
            // Failure case: expected error but got success
            Expectation::Error(_) => Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nExpected error, but test succeeded", test_form.name),
                &test_form.source_file,
                test_form.span
            )),
        }
    }

    fn check_error_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual_error: SutraError,
    ) -> Result<(), SutraError> {
        match expected {
            // Success case: error type matches expected
            Expectation::Error(expected_type) if actual_error.error_type() == *expected_type => Ok(()),
            // Failure case: expected error but got different type
            Expectation::Error(expected_type) => Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nExpected error type {}, got {}", test_form.name, expected_type, actual_error.error_type()),
                &test_form.source_file,
                test_form.span
            )),
            // Failure case: expected success but got error
            Expectation::Value(_) => Err(actual_error),
        }
    }

    fn extract_expectation(test_form: &ASTDefinition) -> Result<Expectation, SutraError> {
        // Get expect form from test definition
        let expect = test_form
            .expect_form
            .as_ref()
            .ok_or_else(|| err_msg!(TestSyntaxFailure, "Test is missing expect form"))?;

        // Validate expect form is a list
        let Expr::List(items, _) = &*expect.value else {
            return Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nexpect form must be a list", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        };

        // Look for value or error clause in expect form
        for item in items {
            if let Some(expectation) = Self::extract_clause(item, test_form)? {
                return Ok(expectation);
            }
        }

        Err(err_src!(
            TestSyntaxFailure,
            format!("\"{}\"\nmissing (value <expected>) or (error <type>) in expect form", test_form.name),
            &test_form.source_file,
            test_form.span
        ))
    }

    fn extract_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<Option<Expectation>, SutraError> {
        // Check if item is a list with exactly 2 elements
        let Expr::List(items, _) = &*item.value else {
            return Ok(None);
        };
        if items.len() != 2 {
            return Ok(None);
        };

        // Extract keyword (value/error)
        let Expr::Symbol(keyword, _) = &*items[0].value else {
            return Ok(None);
        };

        // Parse based on keyword type
        match keyword.as_str() {
            "value" => Self::extract_value(&items[1], test_form).map(|v| Some(Expectation::Value(v))),
            "error" => Self::extract_error_type(&items[1], test_form).map(|e| Some(Expectation::Error(e))),
            _ => Ok(None),
        }
    }

    fn extract_value(value_node: &AstNode, test_form: &ASTDefinition) -> Result<Value, SutraError> {
        // Convert AST node to Value based on type
        match &*value_node.value {
            Expr::Number(n, _) => Ok(Value::Number(*n)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::Symbol(s, _) if s == "nil" => Ok(Value::Nil),
            _ => Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nhas unsupported expected value type", test_form.name),
                &test_form.source_file,
                test_form.span
            )),
        }
    }

    fn extract_error_type(error_node: &AstNode, test_form: &ASTDefinition) -> Result<ErrorType, SutraError> {
        // Extract error type symbol
        let Expr::Symbol(error_type, _) = &*error_node.value else {
            return Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nerror type must be a symbol", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        };

        // Map symbol to ErrorType enum
        match error_type.as_str() {
            "Parse" => Ok(ErrorType::Parse),
            "Validation" => Ok(ErrorType::Validation),
            "Eval" => Ok(ErrorType::Eval),
            "TypeError" => Ok(ErrorType::TypeError),
            "Internal" => Ok(ErrorType::Internal),
            "TestFailure" => Ok(ErrorType::TestFailure),
            _ => Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nunknown error type: {}", test_form.name, error_type),
                &test_form.source_file,
                test_form.span
            )),
        }
    }
}

/// Represents expected test outcome (success value or error type)
#[derive(Debug, Clone, PartialEq)]
enum Expectation {
    Value(Value),
    Error(ErrorType),
}
