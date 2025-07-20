use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc, sync::Arc};

use miette::{NamedSource, Report};

use crate::{
    atoms::{AtomRegistry, OutputSink},
    err_ctx, err_msg, err_src,
    macros::{
        expand_macros_recursively, is_macro_definition, parse_macro_definition, MacroDefinition,
        MacroExpansionContext, MacroValidationContext,
    },
    runtime::{
        eval::evaluate,
        world::{build_canonical_macro_env, build_default_atom_registry},
    },
    syntax::parser::{parse, wrap_in_do},
    to_error_source,
    validation::semantic::validate_expanded_ast,
    AstNode, MacroRegistry, SharedOutput, Span, SutraError, Value, World,
};

// ============================================================================
// OUTPUT TYPES - Generic output handling for CLI and testing
// ============================================================================

/// OutputBuffer: collects output into a String for testing or programmatic capture.
pub struct EngineOutputBuffer {
    pub buffer: String,
}

impl EngineOutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.buffer
    }
}

impl Default for EngineOutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for EngineOutputBuffer {
    fn emit(&mut self, text: &str, _span: Option<&Span>) {
        if !self.buffer.is_empty() {
            self.buffer.push('\n');
        }
        self.buffer.push_str(text);
    }
}

/// StdoutSink: writes output to stdout for CLI and default runner use.
pub struct EngineStdoutSink;

impl OutputSink for EngineStdoutSink {
    fn emit(&mut self, text: &str, _span: Option<&Span>) {
        println!("{text}");
    }
}

/// Prints a SutraError with full miette diagnostics
pub fn print_error(error: SutraError) {
    let report = Report::new(error);
    eprintln!("{report:?}");
}

// ============================================================================
// TEST TYPES - Generic test infrastructure
// ============================================================================

/// Test result summary for CLI reporting
#[derive(Debug, Default)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
}

impl TestSummary {
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }

    pub fn total_tests(&self) -> usize {
        self.passed + self.failed
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests() == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total_tests() as f64) * 100.0
    }
}

/// Test result for individual test execution
#[derive(Debug, Clone)]
pub enum TestResult {
    Passed,
    Failed,
}

// ============================================================================
// TYPE ALIASES - Reduce verbosity in engine functions
// ============================================================================

/// Type alias for execution results
type ExecutionResult = Result<(), SutraError>;

/// Type alias for AST execution results
type AstExecutionResult = Result<Value, SutraError>;

/// Type alias for evaluation results with world state
type EvaluationResult = Result<(Value, World), SutraError>;

/// Type alias for source context used in error reporting
type SourceContext = Arc<NamedSource<String>>;

/// Unified execution pipeline that enforces strict layering: Parse → Expand → Validate → Evaluate
/// This is the single source of truth for all Sutra execution paths, including tests and production.
/// All code execution, including test harnesses, must use this pipeline. Bypassing is forbidden.
pub struct ExecutionPipeline {
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        Self {
            max_depth: 100,
            validate: true,
        }
    }
}

impl ExecutionPipeline {
    // ============================================================================
    // HELPER METHODS - Extract common patterns for reuse
    // ============================================================================

    /// Builds standard registries used across all execution paths
    fn build_standard_registries() -> (AtomRegistry, MacroExpansionContext) {
        let atom_registry = build_default_atom_registry();
        let macro_env = build_canonical_macro_env().expect("Standard macro env should build");
        (atom_registry, macro_env)
    }

    /// Creates a source context for error reporting
    fn create_source_context(&self, name: &str, content: &str) -> SourceContext {
        Arc::new(NamedSource::new(name, content.to_string()))
    }

    /// Processes macro definitions and adds them to the environment
    fn process_macro_definitions(
        &self,
        macro_defs: Vec<AstNode>,
        env: &mut MacroExpansionContext,
    ) -> ExecutionResult {
        let ctx = MacroValidationContext::for_user_macros();
        let mut macros = Vec::new();

        for macro_expr in macro_defs {
            let (name, template) = parse_macro_definition(&macro_expr)?;
            macros.push((name, template));
        }

        ctx.validate_and_insert_many(macros, &mut env.user_macros)
    }

    /// Combines core and user macros into a single registry for validation
    fn combine_macro_registries(
        &self,
        env: &MacroExpansionContext,
    ) -> HashMap<String, MacroDefinition> {
        let mut combined = env.core_macros.clone();
        combined.extend(env.user_macros.clone());
        combined
    }

    /// Validates an expanded AST if validation is enabled
    fn validate_expanded_ast(
        &self,
        expanded: &AstNode,
        env: &MacroExpansionContext,
        atom_registry: &AtomRegistry,
        source_context: &SourceContext,
    ) -> ExecutionResult {
        if !self.validate {
            return Ok(());
        }

        let combined_macros = self.combine_macro_registries(env);
        let macro_registry = MacroRegistry {
            macros: combined_macros,
        };

        let validation_result = validate_expanded_ast(expanded, &macro_registry, atom_registry);

        if !validation_result.is_valid() {
            let error_message = validation_result.errors.join("\n");
            return Err(err_ctx!(
                Validation,
                format!("Semantic validation failed:\n{}", error_message),
                source_context,
                expanded.span,
                "Check for undefined symbols, type errors, or invalid macro usage in your code."
            ));
        }

        Ok(())
    }

    /// Evaluates an expanded AST with standard context
    fn evaluate_expanded_ast(
        &self,
        expanded: &AstNode,
        output: SharedOutput,
        source_context: SourceContext,
        atom_registry: &AtomRegistry,
    ) -> EvaluationResult {
        let world = World::default();
        evaluate(
            expanded,
            &world,
            output,
            atom_registry,
            source_context,
            self.max_depth,
        )
    }

    /// Outputs result if not nil
    fn output_result_if_not_nil(&self, result: &Value, output: &SharedOutput) {
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }
    }

    // ============================================================================
    // CLI SERVICE METHODS - Pure execution services for CLI orchestration
    // ============================================================================

    /// Executes source code with pure execution logic (no I/O, no formatting)
    pub fn execute_source(source: &str, output: SharedOutput) -> Result<(), SutraError> {
        Self::default().execute(source, output)
    }

    /// Parses source code with pure parsing logic (no I/O)
    pub fn parse_source(source: &str) -> Result<Vec<AstNode>, SutraError> {
        parse(source)
    }

    /// Expands macros in source code with pure expansion logic (no I/O)
    pub fn expand_macros_source(source: &str) -> Result<String, SutraError> {
        let ast_nodes = parse(source)?;
        let mut env = build_canonical_macro_env()?;
        let program = wrap_in_do(ast_nodes);
        let expanded = expand_macros_recursively(program, &mut env)?;
        Ok(expanded.value.pretty())
    }

    /// Reads a file with standardized error handling
    pub fn read_file(path: &Path) -> Result<String, SutraError> {
        let filename = path
            .to_str()
            .ok_or_else(|| err_msg!(Internal, "Invalid filename"))?;

        std::fs::read_to_string(filename).map_err(|error| {
            err_ctx!(
                Internal,
                format!("Failed to read file: {}", error),
                &to_error_source(filename),
                Span::default(),
                "Check that the file exists and is readable."
            )
        })
    }

    // ============================================================================
    // TEST EXECUTION SERVICES - Pure test logic for CLI
    // ============================================================================

    /// Executes a test with expectation checking (pure logic, no I/O)
    pub fn execute_test_with_expectation(
        test_form: &crate::discovery::ASTDefinition,
    ) -> TestResult {
        let expected_value = match Self::extract_expect_value(test_form) {
            Ok(value) => value,
            Err(_) => return TestResult::Failed,
        };

        if Self::is_error_test(&expected_value) {
            return Self::handle_error_test(test_form, &expected_value);
        }

        Self::handle_value_test(test_form, &expected_value)
    }

    /// Checks if the expected value indicates an error test
    fn is_error_test(expected_value: &Value) -> bool {
        matches!(expected_value, Value::String(s) if s.starts_with("ERROR:"))
    }

    /// Handles error test execution
    fn handle_error_test(
        test_form: &crate::discovery::ASTDefinition,
        expected_value: &Value,
    ) -> TestResult {
        let expected_error_type = match expected_value {
            Value::String(s) => &s[6..], // Remove "ERROR:" prefix
            _ => return TestResult::Failed,
        };

        match Self::execute_test_body(&test_form.body) {
            Ok(_) => TestResult::Failed, // Expected to fail but succeeded
            Err(error) => {
                let error_type = Self::extract_error_type(&error);
                if error_type == expected_error_type {
                    TestResult::Passed
                } else {
                    TestResult::Failed
                }
            }
        }
    }

    /// Handles value test execution
    fn handle_value_test(
        test_form: &crate::discovery::ASTDefinition,
        expected_value: &Value,
    ) -> TestResult {
        let actual_value = match Self::execute_test_body(&test_form.body) {
            Ok(value) => value,
            Err(_) => return TestResult::Failed,
        };

        if actual_value == *expected_value {
            TestResult::Passed
        } else {
            TestResult::Failed
        }
    }

    /// Executes test body with pure execution logic
    fn execute_test_body(body: &[AstNode]) -> Result<Value, SutraError> {
        if body.is_empty() {
            return Ok(Value::Nil);
        }

        let pipeline = Self::default();
        pipeline.execute_ast(body)
    }

    /// Extracts expected value from test form
    fn extract_expect_value(
        test_form: &crate::discovery::ASTDefinition,
    ) -> Result<Value, SutraError> {
        let expect = test_form
            .expect_form
            .as_ref()
            .ok_or_else(|| err_msg!(TestFailure, "Test is missing expect form"))?;

        let crate::ast::Expr::List(items, _) = &*expect.value else {
            return Err(err_src!(
                TestFailure,
                format!("Test '{}' expect form must be a list", test_form.name),
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
            TestFailure,
            format!(
                "Test '{}' missing (value <expected>) or (error <type>) in expect form",
                test_form.name
            ),
            &test_form.source_file,
            test_form.span
        ))
    }

    /// Extracts value clause from test item
    fn extract_value_clause(
        item: &AstNode,
        test_form: &crate::discovery::ASTDefinition,
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
                TestFailure,
                format!(
                    "Test '{}' has unsupported expected value type",
                    test_form.name
                ),
                &test_form.source_file,
                test_form.span
            )),
        }
    }

    /// Extracts error clause from test item
    fn extract_error_clause(
        item: &AstNode,
        test_form: &crate::discovery::ASTDefinition,
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
                TestFailure,
                format!("Test '{}' error type must be a symbol", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        };

        Ok(Some(Value::String(format!("ERROR:{}", error_type))))
    }

    /// Extracts error type from SutraError
    fn extract_error_type(error: &SutraError) -> String {
        match error {
            SutraError::Parse { .. } => "Parse".to_string(),
            SutraError::Validation { .. } => "Validation".to_string(),
            SutraError::Eval { .. } => "Eval".to_string(),
            SutraError::TypeError { .. } => "TypeError".to_string(),
            SutraError::DivisionByZero { .. } => "Eval".to_string(),
            SutraError::Internal { .. } => "Internal".to_string(),
            SutraError::TestFailure { .. } => "TestFailure".to_string(),
        }
    }

    // ============================================================================
    // REGISTRY ACCESS SERVICES - Pure registry access for CLI
    // ============================================================================

    /// Gets the atom registry (pure access, no I/O)
    pub fn get_atom_registry() -> AtomRegistry {
        build_default_atom_registry()
    }

    /// Gets the macro registry (pure access, no I/O)
    pub fn get_macro_registry() -> MacroExpansionContext {
        build_canonical_macro_env().expect("Standard macro env should build")
    }

    /// Lists all available atoms (pure access, no I/O)
    pub fn list_atoms() -> Vec<String> {
        let atom_registry = Self::get_atom_registry();
        atom_registry.atoms.keys().cloned().collect()
    }

    /// Lists all available macros (pure access, no I/O)
    pub fn list_macros() -> Vec<String> {
        let macro_registry = Self::get_macro_registry();
        let mut items = Vec::new();
        items.extend(macro_registry.core_macros.keys().cloned());
        items.extend(macro_registry.user_macros.keys().cloned());
        items
    }

    // ============================================================================
    // PUBLIC EXECUTION METHODS
    // ============================================================================

    /// Executes Sutra source code through the complete pipeline.
    /// This is the single entry point for all execution paths, including tests.
    pub fn execute(&self, source: &str, output: SharedOutput) -> ExecutionResult {
        // Phase 1: Parse the source into AST nodes
        let ast_nodes = parse(source)?;

        // Phase 2: Partition AST nodes: macro definitions vs user code
        // Note: define forms are special forms that should be evaluated, not treated as macros
        let (macro_defs, user_code): (Vec<_>, Vec<_>) =
            ast_nodes.into_iter().partition(|_expr| false); // Don't partition define forms as macros

        // Phase 3: Build canonical macro environment
        let mut env = build_canonical_macro_env()?;

        // Phase 4: Process user-defined macros
        self.process_macro_definitions(macro_defs, &mut env)?;

        // Phase 5: Wrap user_code in a (do ...) if needed
        let program = wrap_in_do(user_code);

        // Phase 6: Expand macros (CRITICAL: This happens BEFORE evaluation)
        let expanded = expand_macros_recursively(program, &mut env)?;

        // Phase 7: Validation step (optional but recommended)
        let (atom_registry, _) = Self::build_standard_registries();
        let source_context = self.create_source_context("source", source);
        self.validate_expanded_ast(&expanded, &env, &atom_registry, &source_context)?;

        // Phase 8: Evaluate the expanded AST (CRITICAL: No macro expansion happens here)
        let (result, _updated_world) =
            self.evaluate_expanded_ast(&expanded, output.clone(), source_context, &atom_registry)?;

        // Phase 9: Output result (if not nil)
        self.output_result_if_not_nil(&result, &output);

        Ok(())
    }

    /// Executes a single AST node that has already been expanded.
    /// This is used for testing and internal evaluation where macro expansion
    /// has already been performed.
    pub fn execute_expanded_ast(
        &self,
        expanded_ast: &AstNode,
        world: &World,
        output: SharedOutput,
        source: SourceContext,
    ) -> Result<(Value, World), SutraError> {
        let atom_registry = build_default_atom_registry();
        evaluate(
            expanded_ast,
            world,
            output,
            &atom_registry,
            source,
            self.max_depth,
        )
    }

    /// Executes test code with proper macro expansion and special form preservation.
    /// This method is specifically designed for test execution and ensures that
    /// both macro expansion and special form evaluation work correctly.
    pub fn execute_test(&self, test_body: &AstNode, output: SharedOutput) -> ExecutionResult {
        // Phase 1: Build canonical macro environment (includes null?, etc.)
        let mut env = build_canonical_macro_env()?;

        // Phase 2: Expand macros in the test body
        let expanded = expand_macros_recursively(test_body.clone(), &mut env)?;

        // Phase 3: Evaluate the expanded AST
        let (atom_registry, _) = Self::build_standard_registries();
        let source_context = self.create_source_context("test", "");
        let (result, _updated_world) =
            self.evaluate_expanded_ast(&expanded, output.clone(), source_context, &atom_registry)?;

        // Phase 4: Output result (if not nil)
        self.output_result_if_not_nil(&result, &output);

        Ok(())
    }

    /// Executes AST nodes directly without parsing, avoiding double execution.
    /// This is optimized for test execution where AST is already available.
    pub fn execute_ast(&self, nodes: &[AstNode]) -> AstExecutionResult {
        // Partition AST nodes: macro definitions vs user code
        let (macro_defs, user_code) = nodes.iter().cloned().partition(is_macro_definition);

        // Build canonical macro environment
        let mut env = build_canonical_macro_env()?;

        // Process user-defined macros
        self.process_macro_definitions(macro_defs, &mut env)?;

        // Wrap user_code in a (do ...) if needed
        let program = wrap_in_do(user_code);

        // Expand macros
        let expanded = expand_macros_recursively(program, &mut env)?;

        // Optional validation step
        let (atom_registry, _) = Self::build_standard_registries();
        let source_context = self.create_source_context("ast_execution", "");
        self.validate_expanded_ast(&expanded, &env, &atom_registry, &source_context)?;

        // Evaluate the expanded AST
        let output = SharedOutput(Rc::new(RefCell::new(EngineOutputBuffer::new())));
        let (result, _) =
            self.evaluate_expanded_ast(&expanded, output, source_context, &atom_registry)?;

        Ok(result)
    }

    /// Run a single test case, returning Ok(()) if passed, or Err(error) if failed.
    pub fn run_single_test(test_form: &crate::discovery::ASTDefinition) -> Result<(), SutraError> {
        let expected = Self::extract_expect_value(test_form)?;
        if Self::is_error_test(&expected) {
            match Self::execute_test_body(&test_form.body) {
                Ok(_) => Err(err_src!(
                    TestFailure,
                    "Expected error, but test succeeded",
                    &test_form.source_file,
                    test_form.span
                )),
                Err(e) => {
                    let error_type = Self::extract_error_type(&e);
                    let expected_type = match expected {
                        crate::ast::value::Value::String(ref s) => &s[6..],
                        _ => "",
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
                    TestFailure,
                    format!("Expected {}, got {}", expected, actual),
                    &test_form.source_file,
                    test_form.span
                )),
                Err(e) => Err(e),
            }
        }
    }
}
