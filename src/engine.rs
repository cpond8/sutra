use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use miette::NamedSource;

use crate::{
    atoms::AtomRegistry,
    err_ctx,
    macros::{
        expand_macros_recursively, is_macro_definition, parse_macro_definition, MacroDefinition,
        MacroExpansionContext, MacroValidationContext,
    },
    runtime::{
        eval::evaluate,
        world::{build_canonical_macro_env, build_default_atom_registry},
    },
    syntax::parser::{parse, wrap_in_do},
    validation::semantic::validate_expanded_ast,
    AstNode, MacroRegistry, OutputBuffer, SharedOutput, SutraError, Value, World,
};

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
        let output = SharedOutput(Rc::new(RefCell::new(OutputBuffer::new())));
        let (result, _) =
            self.evaluate_expanded_ast(&expanded, output, source_context, &atom_registry)?;

        Ok(result)
    }
}
