use std::{collections::HashMap, path::Path, sync::Arc};

use miette::{NamedSource, Report};

use crate::prelude::*;
use crate::{
    atoms::{
        build_canonical_macro_env, build_canonical_world, special_forms::call_lambda, OutputSink,
        SharedOutput,
    },
    errors::{
        self, DiagnosticInfo, ErrorKind, ErrorReporting, FileContext, SourceContext, SourceInfo,
        SutraError,
    },
    macros::{
        expand_macros_recursively, parse_macro_definition, MacroDefinition, MacroExpansionContext,
        MacroValidationContext,
    },
    runtime::{SpannedResult, SpannedValue},
    syntax::{parser, ConsCell},
    validation::{semantic, ValidationContext},
};

// ============================================================================
// EVALUATION CONTEXT - Simplified evaluation state
// ============================================================================

/// Simplified evaluation context with essential state only
pub struct EvaluationContext {
    pub world: CanonicalWorld,
    pub output: SharedOutput,
    pub source: SourceContext,
    pub depth: usize,
    pub max_depth: usize,
    pub env: HashMap<String, Value>, // Single environment instead of stack
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(world: CanonicalWorld, output: SharedOutput, source: SourceContext) -> Self {
        let mut env = HashMap::new();
        env.insert("nil".to_string(), Value::Nil);

        Self {
            world,
            output,
            source,
            depth: 0,
            max_depth: 1000,
            env,
        }
    }

    /// Create context with custom settings
    pub fn with_settings(
        world: CanonicalWorld,
        output: SharedOutput,
        source: SourceContext,
        max_depth: usize,
    ) -> Self {
        let mut ctx = Self::new(world, output, source);
        ctx.max_depth = max_depth;
        ctx
    }

    /// Set a variable in the environment
    pub fn set_var(&mut self, name: &str, value: Value) {
        self.env.insert(name.to_string(), value);
    }

    /// Get a variable from the environment
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        self.env.get(name)
    }

    /// Create a new lexical frame for let/lambda
    pub fn with_new_frame(&self) -> Self {
        let mut new = Self {
            world: Rc::clone(&self.world),
            output: self.output.clone(),
            source: self.source.clone(),
            depth: self.depth,
            max_depth: self.max_depth,
            env: self.env.clone(),
        };
        new.env.insert("nil".to_string(), Value::Nil);
        new
    }

    /// Extract span information for error reporting
    pub fn span_for_node(&self, node: &AstNode) -> miette::SourceSpan {
        crate::errors::to_source_span(node.span)
    }
}

impl ErrorReporting for EvaluationContext {
    fn report(&self, kind: ErrorKind, span: miette::SourceSpan) -> SutraError {
        SutraError {
            kind: kind.clone(),
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::Runtime { test_info: None },
            },
            diagnostic_info: DiagnosticInfo {
                help: None,
                related_spans: Vec::new(),
                error_code: format!("sutra::engine::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}

// ============================================================================
// CORE EVALUATION - Simplified evaluation engine
// ============================================================================

/// Main evaluation entry point
pub fn evaluate(
    expr: &AstNode,
    world: CanonicalWorld,
    output: SharedOutput,
    source: SourceContext,
) -> Result<Value, SutraError> {
    let mut context = EvaluationContext::new(world, output, source);
    let result = evaluate_ast_node(expr, &mut context)?;
    Ok(result.value)
}

/// Core recursive evaluator
pub(crate) fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> SpannedResult {
    // Check recursion limit
    if context.depth > context.max_depth {
        return Err(context.report(ErrorKind::RecursionLimit, context.span_for_node(expr)));
    }

    // Evaluate based on expression type
    match &*expr.value {
        Expr::List(items, _) => evaluate_call(items, context),
        Expr::Quote(inner, _) => Ok(SpannedValue {
            value: Value::Quote(Box::new(ast_to_value(inner))),
            span: expr.span,
        }),
        Expr::Symbol(name, _) => resolve_symbol(name, expr, context),
        Expr::String(s, _) => Ok(SpannedValue {
            value: Value::String(s.clone()),
            span: expr.span,
        }),
        Expr::Number(n, _) => Ok(SpannedValue {
            value: Value::Number(*n),
            span: expr.span,
        }),
        Expr::Bool(b, _) => Ok(SpannedValue {
            value: Value::Bool(*b),
            span: expr.span,
        }),
        Expr::Path(p, _) => Ok(SpannedValue {
            value: Value::Path(p.clone()),
            span: expr.span,
        }),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let is_true = evaluate_condition_as_bool(condition, context)?;
            if is_true {
                evaluate_ast_node(then_branch, context)
            } else {
                evaluate_ast_node(else_branch, context)
            }
        }
        _ => Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "evaluate".to_string(),
                operand_type: expr.value.type_name().to_string(),
            },
            context.span_for_node(expr),
        )),
    }
}

/// Evaluate function calls
fn evaluate_call(items: &[AstNode], context: &mut EvaluationContext) -> SpannedResult {
    if items.is_empty() {
        return Ok(SpannedValue {
            value: Value::Nil,
            span: items.first().map(|n| n.span).unwrap_or_default(),
        });
    }

    let head = &items[0];
    let tail = &items[1..];

    // Resolve callable
    let callable = if let Expr::Symbol(name, _) = &*head.value {
        resolve_symbol(name, head, context)?.value
    } else {
        evaluate_ast_node(head, context)?.value
    };

    // Dispatch call
    match callable {
        Value::Lambda(lambda) => {
            // Evaluate arguments for lambda calls
            let args = evaluate_args(tail, context)?;
            call_lambda(&lambda, &args, context, &head.span)
        }
        Value::NativeFn(func) => {
            // Pass unevaluated arguments to native function
            func(tail, context, &head.span)
        }
        _ => Err(context.report(
            ErrorKind::TypeMismatch {
                expected: "callable".to_string(),
                actual: callable.type_name().to_string(),
            },
            context.span_for_node(head),
        )),
    }
}

/// Evaluate arguments for function calls
fn evaluate_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<Vec<Value>, SutraError> {
    let mut values = Vec::new();
    for arg in args {
        let result = evaluate_ast_node(arg, context)?;
        values.push(result.value);
    }
    Ok(values)
}

/// Resolve symbol to value
fn resolve_symbol(name: &str, node: &AstNode, context: &mut EvaluationContext) -> SpannedResult {
    // Check local environment first
    if let Some(value) = context.get_var(name) {
        return Ok(SpannedValue {
            value: value.clone(),
            span: node.span,
        });
    }

    // Check global world state
    let world_path = Path(vec![name.to_string()]);
    if let Some(value) = context.world.borrow().state.get(&world_path) {
        return Ok(SpannedValue {
            value: value.clone(),
            span: node.span,
        });
    }

    // Undefined
    Err(context.report(
        ErrorKind::UndefinedSymbol {
            symbol: name.to_string(),
        },
        context.span_for_node(node),
    ))
}

/// Convert AST to quoted value
fn ast_to_value(node: &AstNode) -> Value {
    match &*node.value {
        Expr::Symbol(s, _) => Value::Symbol(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::List(items, _) => {
            let mut result = Value::Nil;
            for item in items.iter().rev() {
                let cell = ConsCell {
                    car: ast_to_value(item),
                    cdr: result,
                };
                result = Value::Cons(Rc::new(cell));
            }
            result
        }
        Expr::Quote(inner, _) => Value::Quote(Box::new(ast_to_value(inner))),
        Expr::Path(p, _) => Value::Path(p.clone()),
        _ => Value::Nil,
    }
}

/// Evaluate condition as boolean
pub fn evaluate_condition_as_bool(
    condition: &AstNode,
    context: &mut EvaluationContext,
) -> Result<bool, SutraError> {
    let result = evaluate_ast_node(condition, context)?;
    Ok(is_truthy(&result.value))
}

/// Check if value is truthy
fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Bool(false) => false,
        Value::Nil => false,
        Value::Number(n) => *n != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Map(m) => !m.is_empty(),
        Value::Quote(inner) => is_truthy(inner),
        _ => true,
    }
}

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
        let mut env = build_canonical_macro_env()?;

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
        let validation_errors =
            semantic::validate_ast_semantics(expanded, &macro_registry, world, source_context);

        // Step 4: Test context is handled automatically by EvaluationContext
        // when it creates errors, so no post-processing needed here

        // TODO: Ensure Engine passes test_file/test_name to EvaluationContext
        // when creating the evaluation context, rather than modifying errors afterward

        // Step 5: Handle validation results
        if !validation_errors.is_empty() {
            // Return the first validation error. The validator now creates fully-contextualized errors,
            // so we no longer need to wrap them in a generic error.
            return Err(validation_errors.into_iter().next().unwrap());
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
            world: build_canonical_world(),
            macro_env: build_canonical_macro_env().unwrap_or_else(|e| {
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
            let context = ValidationContext {
                source: SourceContext::fallback("ExecutionPipeline::read_file"),
                phase: "file-system".to_string(),
            };
            context.report(
                ErrorKind::InvalidPath {
                    path: path.to_string_lossy().to_string(),
                },
                errors::unspanned(),
            )
        })?;

        std::fs::read_to_string(filename).map_err(|error| {
            let context = ValidationContext {
                source: SourceContext::fallback("ExecutionPipeline::read_file"),
                phase: "file-system".to_string(),
            };
            context.report(
                ErrorKind::InvalidPath {
                    path: format!("{} ({})", filename, error),
                },
                errors::unspanned(),
            )
        })
    }

    // ============================================================================
    // REGISTRY ACCESS SERVICES - Pure registry access for CLI
    // ============================================================================

    /// Gets the world state (pure access, no I/O)

    /// Gets the macro registry (pure access, no I/O)
    pub fn get_macro_registry() -> MacroExpansionContext {
        build_canonical_macro_env().expect("Standard macro env should build")
    }

    /// Lists all available atoms (pure access, no I/O)
    pub fn list_atoms() -> Vec<String> {
        let world = build_canonical_world();
        let world = world.borrow();
        if let Some(Value::Map(map)) = world.state.get(&crate::atoms::Path(vec![])) {
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
        evaluate(&expanded, self.world.clone(), output, source_context)
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
        source: SourceContext,
    ) -> Result<Value, SutraError> {
        evaluate(expanded_ast, world, output, source)
    }
}
