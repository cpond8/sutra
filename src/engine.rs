use std::{collections::HashMap, path::Path, sync::Arc};

use miette::{NamedSource, Report};

use crate::prelude::*;
use crate::{
    atoms::{OutputSink, SharedOutput},
    macros::{
        expand_macros_recursively, parse_macro_definition, MacroDefinition, MacroExpansionContext,
        MacroValidationContext,
    },
    runtime::{self, eval, source, world},
    syntax::parser,
    validation::semantic,
};

use crate::errors;
use miette::SourceSpan;

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
// TYPE ALIASES - Reduce verbosity in engine functions
// ============================================================================

/// Type alias for execution results
type ExecutionResult = Result<(), SutraError>;

/// Type alias for source context used in error reporting

// ============================================================================
// MACRO PROCESSOR - Dedicated macro processing service to eliminate duplication
// ============================================================================

/// Dedicated macro processing service that encapsulates macro environment building and processing.
pub struct MacroProcessor {
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
    /// Test file name for error reporting
    pub test_file: Option<String>,
    /// Test name for error reporting
    pub test_name: Option<String>,
}

impl Default for MacroProcessor {
    fn default() -> Self {
        Self {
            validate: true,
            max_depth: 100,
            test_file: None,
            test_name: None,
        }
    }
}

impl MacroProcessor {
    /// Creates a new macro processor with specified settings
    pub fn new(validate: bool, max_depth: usize) -> Self {
        Self {
            validate,
            max_depth,
            test_file: None,
            test_name: None,
        }
    }

    /// Creates a new macro processor with test context
    pub fn with_test_context(mut self, test_file: String, test_name: String) -> Self {
        self.test_file = Some(test_file);
        self.test_name = Some(test_name);
        self
    }

    /// Partition AST nodes, process macros, and expand - unified macro processing pipeline
    pub fn partition_and_process_macros(
        &self,
        ast_nodes: Vec<AstNode>,
    ) -> Result<(AstNode, MacroExpansionContext), SutraError> {
        // Step 1: Partition AST nodes into macro definitions and user code
        let (macro_defs, user_code) = self.partition_ast_nodes(ast_nodes);

        // Step 2: Build canonical macro environment
        let mut env = world::build_canonical_macro_env()?;

        // Step 3: Process user-defined macros
        self.process_macro_definitions(macro_defs, &mut env)?;

        // Step 4: Wrap user code in a (do ...) block if needed
        let program = parser::wrap_in_do(user_code);

        // Step 5: Expand macros recursively
        let expanded = expand_macros_recursively(program, &mut env)?;

        Ok((expanded, env))
    }

    /// Process with existing macro environment, expanding macros and returning the result.
    /// This is ideal for test runners or other contexts where a pre-configured macro
    /// environment is available.
    pub fn process_with_existing_macros(
        &self,
        ast_nodes: Vec<AstNode>,
        env: &mut MacroExpansionContext,
    ) -> Result<AstNode, SutraError> {
        // Step 1: Partition AST nodes into macro definitions and user code
        let (macro_defs, user_code) = self.partition_ast_nodes(ast_nodes);

        // Step 2: Process user-defined macros into the existing environment
        self.process_macro_definitions(macro_defs, env)?;

        // Step 3: Wrap user code in a (do ...) block if needed
        let program = parser::wrap_in_do(user_code);

        // Step 4: Expand macros recursively
        expand_macros_recursively(program, env)
    }

    /// Partitions AST nodes into macro definitions and user code
    fn partition_ast_nodes(&self, ast_nodes: Vec<AstNode>) -> (Vec<AstNode>, Vec<AstNode>) {
        // Note: define forms are special forms that should be evaluated, not treated as macros
        ast_nodes.into_iter().partition(|_expr| false) // Don't partition define forms as macros
    }

    /// Expand macros with existing environment (for single AST nodes)
    pub fn expand_with_macros(
        &self,
        ast: AstNode,
        env: &mut MacroExpansionContext,
    ) -> Result<AstNode, SutraError> {
        expand_macros_recursively(ast, env)
    }

    /// Processes macro definitions and adds them to the environment
    pub fn process_macro_definitions(
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
    pub fn combine_macro_registries(
        &self,
        env: &MacroExpansionContext,
    ) -> HashMap<String, MacroDefinition> {
        let mut combined = env.core_macros.clone();
        combined.extend(env.user_macros.clone());
        combined
    }

    /// Validates an expanded AST if validation is enabled
    pub fn validate_expanded_ast(
        &self,
        expanded: &AstNode,
        env: &MacroExpansionContext,
        world: &World,
        source_context: &source::SourceContext,
    ) -> ExecutionResult {
        // Step 1: Check if validation is enabled
        if !self.validate {
            return Ok(());
        }

        // Step 2: Build macro registry for validation
        let combined_macros = self.combine_macro_registries(env);
        let macro_registry = MacroRegistry {
            macros: combined_macros,
        };

        // Step 3: Perform semantic validation
        let mut validation_result =
            semantic::validate_expanded_ast(expanded, &macro_registry, world, source_context);

        // Step 4: Add test context to any validation errors
        if let (Some(ref tf), Some(ref tn)) = (&self.test_file, &self.test_name) {
            validation_result.errors = validation_result
                .errors
                .into_iter()
                .map(|error| error.with_test_context(tf.clone(), tn.clone()))
                .collect();
        }

        // Step 5: Handle validation results
        if !validation_result.is_valid() {
            // Return the first validation error. The validator now creates fully-contextualized errors,
            // so we no longer need to wrap them in a generic error.
            return Err(validation_result.errors.remove(0));
        }

        // Step 6: Validation successful
        Ok(())
    }
}

// ============================================================================
// EXECUTION PIPELINE - Unified execution using MacroProcessor
// ============================================================================

/// Unified execution pipeline that enforces strict layering: Parse → Expand → Validate → Evaluate
/// This is the single source of truth for all Sutra execution paths, including tests and production.
/// All code execution, including test harnesses, must use this pipeline. Bypassing is forbidden.
pub struct ExecutionPipeline {
    /// Macro environment with canonical macros pre-loaded.
    pub world: CanonicalWorld,
    /// Macro environment with canonical macros pre-loaded.
    pub macro_env: MacroExpansionContext,
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        Self {
            world: runtime::build_canonical_world(),
            macro_env: world::build_canonical_macro_env().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load standard macros: {}", e);
                // Create a minimal macro environment with only core macros
                MacroExpansionContext {
                    user_macros: HashMap::new(),
                    core_macros: HashMap::new(),
                    trace: Vec::new(),
                    source: Arc::new(NamedSource::new("fallback", String::new())),
                }
            }),
            max_depth: 100,
            validate: false, // Keep validation disabled for now
        }
    }
}

impl ExecutionPipeline {
    // ============================================================================
    // CLI SERVICE METHODS - Pure execution services for CLI orchestration
    // ============================================================================

    /// Executes source code with pure execution logic (no I/O, no formatting)
    pub fn execute_source(source: &str, output: SharedOutput) -> Result<(), SutraError> {
        Self::default().execute(source, output, "source")
    }

    /// Parses source code with pure parsing logic (no I/O)
    pub fn parse_source(source: &str) -> Result<Vec<AstNode>, SutraError> {
        let source_context = SourceContext::from_file("source", source);
        parser::parse(source, source_context)
    }
    /// Expands macros in source code with pure expansion logic (no I/O)
    pub fn expand_macros_source(source: &str) -> Result<String, SutraError> {
        let processor = MacroProcessor::default();
        let source_context = SourceContext::from_file("source", source);
        let ast_nodes = parser::parse(source, source_context)?;
        let (expanded, _env) = processor.partition_and_process_macros(ast_nodes)?;
        Ok(expanded.value.pretty())
    }

    /// Reads a file with standardized error handling
    pub fn read_file(path: &Path) -> Result<String, SutraError> {
        let filename = path.to_str().ok_or_else(|| {
            let sc = SourceContext::fallback("ExecutionPipeline::read_file");
            errors::runtime_general(
                "Invalid filename: Could not convert path to string",
                "file error",
                &sc,
                SourceSpan::from(0..0), // No precise span available in file system error context.
            )
        })?;

        std::fs::read_to_string(filename).map_err(|error| {
            let sc = source::SourceContext::fallback("ExecutionPipeline::read_file");
            errors::runtime_general(
                format!("Failed to read file: {}", error),
                "file error",
                &sc,
                SourceSpan::from(0..0), // No precise span available in file system error context.
            )
        })
    }

    // ============================================================================
    // REGISTRY ACCESS SERVICES - Pure registry access for CLI
    // ============================================================================

    /// Gets the world state (pure access, no I/O)

    /// Gets the macro registry (pure access, no I/O)
    pub fn get_macro_registry() -> MacroExpansionContext {
        world::build_canonical_macro_env().expect("Standard macro env should build")
    }

    /// Lists all available atoms (pure access, no I/O)
    pub fn list_atoms() -> Vec<String> {
        let world = runtime::build_canonical_world();
        let world = world.borrow();
        if let Some(Value::Map(map)) = world.state.get(&world::Path(vec![])) {
            map.keys().cloned().collect()
        } else {
            vec![]
        }
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

    /// Core execution method that all execution variants use.
    /// Takes AST nodes and returns the result value after complete pipeline processing.
    /// This is the single source of truth for AST execution.
    /// Core execution method that processes AST nodes through the full pipeline.
    pub fn execute_nodes(
        &self,
        nodes: &[AstNode],
        output: SharedOutput,
        source_context: SourceContext,
    ) -> Result<Value, SutraError> {
        // Step 1: Create a macro processor with the pipeline's configuration.
        let processor = MacroProcessor::new(self.validate, self.max_depth);
        let mut env = self.macro_env.clone();

        // Step 2: Expand macros using the pipeline's environment.
        let expanded = processor.process_with_existing_macros(nodes.to_vec(), &mut env)?;

        // Step 3: Validate the expanded AST.
        processor.validate_expanded_ast(&expanded, &env, &self.world.borrow(), &source_context)?;

        // Step 4: Evaluate the final AST, using the pipeline's world and output sink.
        eval::evaluate(
            &expanded,
            self.world.clone(),
            output,
            source_context,
            self.max_depth,
            None,
            None,
        )
    }

    /// Executes Sutra source code through the complete pipeline.
    /// This parses source then calls execute_nodes() for unified processing.
    pub fn execute(
        &self,
        source_text: &str,
        output: SharedOutput,
        filename: &str,
    ) -> ExecutionResult {
        // Step 1: Create a source context from the raw text.
        let source_context = SourceContext::from_file(filename, source_text);

        // Step 2: Parse the source into AST nodes.
        let ast_nodes = parser::parse(source_text, source_context.clone())?;

        // Step 3: Execute the nodes through the unified pipeline.
        let result = self.execute_nodes(&ast_nodes, output.clone(), source_context)?;

        // Step 4: Emit the final result to the output sink if it's not nil.
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    /// Executes already-expanded AST nodes, bypassing macro processing.
    /// This is optimized for test execution where AST is already available.
    pub fn execute_expanded_ast(
        &self,
        expanded_ast: &AstNode,
        world: CanonicalWorld,
        output: SharedOutput,
        source: source::SourceContext,
    ) -> Result<Value, SutraError> {
        eval::evaluate(
            expanded_ast,
            world,
            output,
            source,
            self.max_depth,
            None,
            None,
        )
    }
}
