use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc, sync::Arc};

use miette::{NamedSource, Report};

use crate::prelude::*;
use crate::{
    atoms::{OutputSink, SharedOutput},
    macros::{
        expand_macros_recursively, parse_macro_definition, MacroDefinition, MacroExpansionContext,
        MacroValidationContext,
    },
    runtime::{eval, world, source::SourceContext as RuntimeSourceContext},
    syntax::parser,
    validation::semantic,
};

use miette::SourceSpan;
use crate::errors;

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
// TYPE ALIASES - Reduce verbosity in engine functions
// ============================================================================

/// Type alias for execution results
type ExecutionResult = Result<(), SutraError>;

/// Type alias for evaluation results with world state
type EvaluationResult = Result<Value, SutraError>;

/// Type alias for source context used in error reporting
type SourceContext = Arc<NamedSource<String>>;

// ============================================================================
// MACRO PROCESSOR - Dedicated macro processing service to eliminate duplication
// ============================================================================

/// Dedicated macro processing service that encapsulates macro environment building and processing.
pub struct MacroProcessor {
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
}

impl Default for MacroProcessor {
    fn default() -> Self {
        Self {
            validate: true,
            max_depth: 100,
        }
    }
}

impl MacroProcessor {
    /// Creates a new macro processor with specified settings
    pub fn new(validate: bool, max_depth: usize) -> Self {
        Self {
            validate,
            max_depth,
        }
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
        atom_registry: &AtomRegistry,
        source_context: &SourceContext,
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
        let validation_result =
            semantic::validate_expanded_ast(expanded, &macro_registry, atom_registry);

        // Step 4: Handle validation results (early return on failure)
        if !validation_result.is_valid() {
            let error_message = validation_result.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            return Err(errors::runtime_general(
                format!("Semantic validation failed:\n{}", error_message),
                "MacroProcessor::validate_expanded_ast".to_string(),
                format!("{:?}", source_context),
                parser::to_source_span(expanded.span),
            ));
        }

        // Step 5: Validation successful
        Ok(())
    }

    /// Evaluates an expanded AST with standard context
    pub fn evaluate_expanded_ast(
        &self,
        expanded: &AstNode,
        output: SharedOutput,
        source_context: SourceContext,
        atom_registry: &AtomRegistry,
    ) -> EvaluationResult {
        let world = Rc::new(RefCell::new(World::default()));
        eval::evaluate(
            expanded,
            world,
            output,
            atom_registry,
            source_context,
            self.max_depth,
            None,
            None,
        )
    }

    /// Evaluates an expanded AST with a provided source context
    pub fn evaluate_expanded_ast_with_context(
        &self,
        expanded: &AstNode,
        output: SharedOutput,
        source_context: Arc<NamedSource<String>>,
        atom_registry: &AtomRegistry,
    ) -> EvaluationResult {
        let world = Rc::new(RefCell::new(world::World::default()));
        eval::evaluate(
            expanded,
            world,
            output,
            atom_registry,
            source_context,
            self.max_depth,
            None,
            None,
        )
    }

    /// Outputs result if not nil
    pub fn output_result_if_not_nil(&self, result: &Value, output: &SharedOutput) {
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }
    }

    /// Builds standard registries used across all execution paths
    pub fn build_standard_registries() -> (AtomRegistry, MacroExpansionContext) {
        let atom_registry = world::build_default_atom_registry();
        let macro_env =
            world::build_canonical_macro_env().expect("Standard macro env should build");
        (atom_registry, macro_env)
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
            world: Rc::new(RefCell::new(World::default())),
            macro_env: world::build_canonical_macro_env()
                .expect("Standard macro env should build"),
            max_depth: 100,
            validate: true,
        }
    }
}

impl ExecutionPipeline {
    // ============================================================================
    // CLI SERVICE METHODS - Pure execution services for CLI orchestration
    // ============================================================================

    /// Executes source code with pure execution logic (no I/O, no formatting)
    pub fn execute_source(source: &str, output: SharedOutput) -> Result<(), SutraError> {
        Self::default().execute(source, output)
    }

    /// Parses source code with pure parsing logic (no I/O)
    pub fn parse_source(source: &str) -> Result<Vec<AstNode>, SutraError> {
        parser::parse(source)
    }
    /// Expands macros in source code with pure expansion logic (no I/O)
    pub fn expand_macros_source(source: &str) -> Result<String, SutraError> {
        let processor = MacroProcessor::default();
        let ast_nodes = parser::parse(source)?;
        let (expanded, _env) = processor.partition_and_process_macros(ast_nodes)?;
        Ok(expanded.value.pretty())
    }

    /// Reads a file with standardized error handling
    pub fn read_file(path: &Path) -> Result<String, SutraError> {
        let filename = path.to_str().ok_or_else(|| {
            errors::runtime_general(
                "Invalid filename: Could not convert path to string".to_string(),
                "ExecutionPipeline::read_file".to_string(),
                file!().to_string(),
                SourceSpan::from(0..0), // No precise span available in file system error context.
            )
        })?;

        std::fs::read_to_string(filename).map_err(|error| {
            errors::runtime_general(
                format!("Failed to read file: {}", error),
                "ExecutionPipeline::read_file".to_string(),
                file!().to_string(),
                SourceSpan::from(0..0), // No precise span available in file system error context.
            )
        })
    }

    // ============================================================================
    // REGISTRY ACCESS SERVICES - Pure registry access for CLI
    // ============================================================================

    /// Gets the atom registry (pure access, no I/O)
    pub fn get_atom_registry() -> AtomRegistry {
        world::build_default_atom_registry()
    }

    /// Gets the macro registry (pure access, no I/O)
    pub fn get_macro_registry() -> MacroExpansionContext {
        world::build_canonical_macro_env().expect("Standard macro env should build")
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

    /// Core execution method that all execution variants use.
    /// Takes AST nodes and returns the result value after complete pipeline processing.
    /// This is the single source of truth for AST execution.
    pub fn execute_nodes(&self, nodes: &[AstNode]) -> Result<Value, SutraError> {
        // Create macro processor with same configuration
        let processor = MacroProcessor::new(self.validate, self.max_depth);
        let mut env = self.macro_env.clone(); // Clone to allow mutation

        // Step 1: Use MacroProcessor to expand with the pre-configured environment
        let expanded = processor.process_with_existing_macros(nodes.to_vec(), &mut env)?;

        // Step 2: Validation step (optional but recommended)
        let (atom_registry, _) = MacroProcessor::build_standard_registries();
        let source_context = RuntimeSourceContext::fallback("engine execution").to_named_source();
        processor.validate_expanded_ast(&expanded, &env, &atom_registry, &source_context)?;

        // Step 3: Evaluate the expanded AST (CRITICAL: No macro expansion happens here)
        let output = SharedOutput(Rc::new(RefCell::new(
            EngineOutputBuffer::new(),
        )));
        let result =
            processor.evaluate_expanded_ast(&expanded, output, source_context, &atom_registry)?;

        Ok(result)
    }

    /// Executes Sutra source code through the complete pipeline.
    /// This parses source then calls execute_nodes() for unified processing.
    pub fn execute(&self, source: &str, output: SharedOutput) -> ExecutionResult {
        // Step 1: Parse the source into AST nodes
        let ast_nodes = parser::parse(source)?;

        // Step 2: Execute nodes through unified pipeline
        let result = self.execute_nodes(&ast_nodes)?;

        // Step 3: Output result (if not nil)
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
        source: SourceContext,
    ) -> Result<Value, SutraError> {
        let atom_registry = world::build_default_atom_registry();
        eval::evaluate(
            expanded_ast,
            world,
            output,
            &atom_registry,
            source,
            self.max_depth,
            None,
            None,
        )
    }

    /// Executes source code with a real NamedSource for error reporting
    pub fn execute_source_with_context(path: &str, source: &str, output: SharedOutput) -> Result<(), SutraError> {
        let named_source = Arc::new(NamedSource::new(path.to_string(), source.to_string()));
        let ast_nodes = parser::parse(source)?;
        let processor = MacroProcessor::default();
        let (atom_registry, mut macro_env) = MacroProcessor::build_standard_registries();
        let expanded_node = processor.process_with_existing_macros(ast_nodes, &mut macro_env)?;
        let source_context = Arc::new(NamedSource::new(path.to_string(), source.to_string()));
        processor.validate_expanded_ast(&expanded_node, &macro_env, &atom_registry, &source_context)?;
        let result = processor.evaluate_expanded_ast_with_context(
            &expanded_node,
            output.clone(),
            named_source.clone(),
            &atom_registry,
        )?;
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }
        Ok(())
    }
}
