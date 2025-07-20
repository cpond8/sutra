use crate::{
    discovery::ASTDefinition, engine::ExecutionPipeline, err_msg, err_src, AstNode, ErrorType,
    SharedOutput, SutraError, Value,
};

/// Executes test code with proper macro expansion and special form preservation.
/// This method is specifically designed for test execution and ensures that
/// both macro expansion and special form evaluation work correctly.
pub struct TestRunner;

impl TestRunner {
    pub fn execute_test(test_body: &AstNode, output: SharedOutput) -> Result<(), SutraError> {
        // Use unified execution pipeline for single AST node
        let pipeline = ExecutionPipeline::default();
        let result = pipeline.execute_nodes(&[test_body.clone()])?;

        // Output result (if not nil)
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    pub fn execute_ast(nodes: &[AstNode]) -> Result<Value, SutraError> {
        // Use unified execution pipeline for AST nodes
        let pipeline = ExecutionPipeline::default();
        pipeline.execute_nodes(nodes)
    }

    pub fn run_single_test(test_form: &ASTDefinition) -> Result<(), SutraError> {
        let expected = Self::extract_expect_value(test_form)?;
        if Self::is_error_test(&expected) {
            match Self::execute_test_body(&test_form.body) {
                Ok(_) => Err(err_src!(
                    TestSyntaxFailure,
                    format!("\"{}\"\nExpected error, but test succeeded", test_form.name),
                    &test_form.source_file,
                    test_form.span
                )),
                Err(e) => {
                    let error_type = e.error_type();
                    let expected_type = match expected {
                        Value::String(ref s) => {
                            // Parse the expected error type from "ERROR:Type" format
                            if s.starts_with("ERROR:") {
                                let type_str = &s[6..];
                                match type_str {
                                    "Parse" => ErrorType::Parse,
                                    "Validation" => ErrorType::Validation,
                                    "Eval" => ErrorType::Eval,
                                    "TypeError" => ErrorType::TypeError,
                                    "Internal" => ErrorType::Internal,
                                    "TestFailure" => ErrorType::TestFailure,
                                    _ => return Err(e), // Unknown error type
                                }
                            } else {
                                return Err(e); // Invalid format
                            }
                        }
                        _ => return Err(e), // Invalid expectation type
                    };
                    if error_type == expected_type {
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            match Self::execute_test_body(&test_form.body) {
                Ok(actual) if actual == expected => Ok(()),
                Ok(actual) => Err(err_src!(
                    TestSyntaxFailure,
                    format!(
                        "\"{}\"\nExpected {}, got {}",
                        test_form.name, expected, actual
                    ),
                    &test_form.source_file,
                    test_form.span
                )),
                Err(e) => Err(e),
            }
        }
    }

    fn is_error_test(expected_value: &Value) -> bool {
        matches!(expected_value, Value::String(s) if s.starts_with("ERROR:"))
    }

    fn execute_test_body(body: &[AstNode]) -> Result<Value, SutraError> {
        if body.is_empty() {
            return Ok(Value::Nil);
        }

        Self::execute_ast(body)
    }

    fn extract_expect_value(test_form: &ASTDefinition) -> Result<Value, SutraError> {
        let expect = test_form
            .expect_form
            .as_ref()
            .ok_or_else(|| err_msg!(TestSyntaxFailure, "Test is missing expect form"))?;

        let crate::ast::Expr::List(items, _) = &*expect.value else {
            return Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nexpect form must be a list", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        };

        // Look for value clause in the expect form
        for item in items {
            if let Some(value) = Self::extract_value_clause(&item, test_form)? {
                return Ok(value);
            }
        }

        // Look for error clause in the expect form
        for item in items {
            if let Some(error_value) = Self::extract_error_clause(&item, test_form)? {
                return Ok(error_value);
            }
        }

        Err(err_src!(
            TestSyntaxFailure,
            format!(
                "\"{}\"\nmissing (value <expected>) or (error <type>) in expect form",
                test_form.name
            ),
            &test_form.source_file,
            test_form.span
        ))
    }

    fn extract_value_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<Option<Value>, SutraError> {
        let crate::ast::Expr::List(value_items, _) = &*item.value else {
            return Ok(None);
        };
        if value_items.len() != 2 {
            return Ok(None);
        };

        let crate::ast::Expr::Symbol(s, _) = &*value_items[0].value else {
            return Ok(None);
        };
        if s != "value" {
            return Ok(None);
        };

        match &*value_items[1].value {
            crate::ast::Expr::Number(n, _) => Ok(Some(Value::Number(*n))),
            crate::ast::Expr::String(s, _) => Ok(Some(Value::String(s.clone()))),
            crate::ast::Expr::Bool(b, _) => Ok(Some(Value::Bool(*b))),
            crate::ast::Expr::Symbol(s, _) if s == "nil" => Ok(Some(Value::Nil)),
            _ => Err(err_src!(
                TestSyntaxFailure,
                format!(
                    "\"{}\"\nhas unsupported expected value type",
                    test_form.name
                ),
                &test_form.source_file,
                test_form.span
            )),
        }
    }

    fn extract_error_clause(
        item: &AstNode,
        test_form: &ASTDefinition,
    ) -> Result<Option<Value>, SutraError> {
        let crate::ast::Expr::List(error_items, _) = &*item.value else {
            return Ok(None);
        };
        if error_items.len() != 2 {
            return Ok(None);
        };

        let crate::ast::Expr::Symbol(s, _) = &*error_items[0].value else {
            return Ok(None);
        };
        if s != "error" {
            return Ok(None);
        };

        let crate::ast::Expr::Symbol(error_type, _) = &*error_items[1].value else {
            return Err(err_src!(
                TestSyntaxFailure,
                format!("\"{}\"\nerror type must be a symbol", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        };

        Ok(Some(Value::String(format!("ERROR:{}", error_type))))
    }
}
