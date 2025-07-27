use std::{cell::RefCell, rc::Rc};

use crate::{
    atoms::SharedOutput,
    discovery::ASTDefinition,
    engine::{EngineOutputBuffer, ExecutionPipeline, MacroProcessor},
    errors,
    prelude::*,
    runtime::{self, eval, source},
    syntax::parser,
};

/// Executes test code with proper macro expansion and special form preservation.
pub struct TestRunner;

impl TestRunner {
    pub fn execute_test(
        test_body: &[AstNode],
        output: SharedOutput,
        test_file: Option<String>,
        test_name: Option<String>,
        source_file: source::SourceContext,
    ) -> Result<(), OldSutraError> {
        // Use a macro processor to handle expansion and evaluation correctly
        let processor = MacroProcessor::default().with_test_context(
            test_file.clone().unwrap_or_default(),
            test_name.clone().unwrap_or_default(),
        );
        let world = runtime::build_canonical_world();
        let mut macro_env =
            runtime::world::build_canonical_macro_env().expect("Standard macro env should build");

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
        let result = eval::evaluate(
            &expanded_node,
            world,
            output.clone(),
            source_context,
            1000,
            test_file,
            test_name,
        )?;

        // If the result is not nil, emit it to the output buffer
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    pub fn execute_ast(
        nodes: &[AstNode],
        source_context: &source::SourceContext,
    ) -> Result<Value, OldSutraError> {
        let pipeline = ExecutionPipeline::default();
        let output = SharedOutput::new(crate::engine::EngineOutputBuffer::new());
        pipeline.execute_nodes(nodes, output, source_context.clone())
    }

    pub fn run_single_test(test_form: &ASTDefinition) -> Result<(), OldSutraError> {
        let source_context = &test_form.source_file;
        let expected = Self::extract_expectation(test_form, source_context)?;

        // Guard: parse error expectation with single string body
        if matches!(&expected, Expectation::Error(ErrorType::Parse))
            && test_form.body.len() == 1
            && matches!(*test_form.body[0].value, Expr::String(_, _))
        {
            let Expr::String(ref code, _) = *test_form.body[0].value else { unreachable!() };
            let parse_result = parser::parse(code, source_context.clone());
            return match parse_result {
                Err(e) if e.error_type() == ErrorType::Parse => Ok(()),
                Err(e) => Err(errors::test_assertion(
                    format!("Expected parse error, but got different error: {:?}", e.error_type()),
                    test_form.name.clone(),
                    source_context,
                    to_source_span(test_form.span),
                )),
                Ok(_) => Err(errors::test_assertion(
                    "Expected parse error, but parsing succeeded",
                    test_form.name.clone(),
                    source_context,
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
                return Err(errors::test_assertion(
                    format!(
                        "Expected output {:?}, got {:?}",
                        expected_output, actual_output_owned
                    ),
                    test_form.name.clone(),
                    source_context,
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
        source_context: &source::SourceContext,
    ) -> Result<(), OldSutraError> {
        match expected {
            // Success case: actual matches expected value
            Expectation::Value(expected_value) if actual == *expected_value => Ok(()),
            // Failure case: actual doesn't match expected value
            Expectation::Value(expected_value) => Err(errors::test_assertion(
                format!("Expected {}, got {}", expected_value, actual),
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
            // Failure case: expected error but got success
            Expectation::Error(_) => Err(errors::test_assertion(
                "Expected error, but test succeeded",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
            // Failure case: output expectation in value test path
            Expectation::Output(_) => Err(errors::test_assertion(
                "Output expectation not handled in value test path. Output expectations should be handled in the output test path",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
        }
    }

    fn check_error_test(
        test_form: &ASTDefinition,
        expected: &Expectation,
        actual_error: OldSutraError,
        source_context: &source::SourceContext,
    ) -> Result<(), OldSutraError> {
        match expected {
            // Success case: error type matches expected
            Expectation::Error(expected_type) if actual_error.error_type() == *expected_type => {
                Ok(())
            }
            // Failure case: expected error but got different type
   Expectation::Error(expected_type) => Err(errors::test_assertion(
                format!(
                    "Expected error type {:?}, got {:?}",
                    expected_type,
                    actual_error.error_type()
                ),
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
            // Failure case: expected success but got error
            Expectation::Value(_) => Err(actual_error),
            // Failure case: output expectation in error test path
            Expectation::Output(_) => Err(errors::test_assertion(
                "Output expectation not handled in error test path. Output expectations should be handled in the output test path",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_expectation(
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<Expectation, OldSutraError> {
        // Get expect form from test definition
        let expect = test_form.expect_form.as_ref().ok_or_else(|| {
            errors::test_assertion(
                "Test is missing expect form",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )
        })?;

        // Validate expect form is a list
        let Expr::List(items, _) = &*expect.value else {
            return Err(errors::test_assertion(
                "expect form must be a list",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            ));
        };

        // Look for value or error clause in expect form
        for item in items {
            if let Some(expectation) = Self::extract_clause(item, test_form, source_context)? {
                return Ok(expectation);
            }
        }

        Err(errors::test_assertion(
            "missing (value <expected>) or (error <type>) in expect form",
            test_form.name.clone(),
            source_context,
            to_source_span(test_form.span),
        ))
    }

    fn extract_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<Option<Expectation>, OldSutraError> {
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
    ) -> Result<Value, OldSutraError> {
        // Convert AST node to Value based on type
        match &*value_node.value {
            Expr::Number(n, _) => Ok(Value::Number(*n)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::Symbol(s, _) if s == "nil" => Ok(Value::Nil),
            Expr::Symbol(s, _) => Ok(Value::Symbol(s.clone())),
            Expr::Quote(inner, _) => Ok(Value::Quote(Box::new(Self::extract_value(inner, test_form, source_context)?))),
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
            _ => Err(errors::test_assertion(
                "unsupported expected value type",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_error_type(
        error_node: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<ErrorType, OldSutraError> {
        // Extract error type symbol
        let Expr::Symbol(error_type, _) = &*error_node.value else {
            return Err(errors::test_assertion(
                "error type must be a symbol",
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
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
            _ => Err(errors::test_assertion(
                format!("unknown error type: {}", error_type),
                test_form.name.clone(),
                source_context,
                to_source_span(test_form.span),
            )),
        }
    }

    fn extract_output(
        output_node: &AstNode,
        test_form: &ASTDefinition,
        source_context: &SourceContext,
    ) -> Result<String, OldSutraError> {
        let Expr::String(s, _) = &*output_node.value else {
            return Err(errors::test_assertion(
                "output expectation must be a string",
                test_form.name.clone(),
                source_context,
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
    Error(ErrorType),
    Output(String),
}
