use std::{cell::RefCell, rc::Rc, sync::Arc};

use miette::NamedSource;

use crate::{
    err_ctx, err_msg,
    macros::{
        expand_macros_recursively, is_macro_definition, parse_macro_definition, MacroDefinition,
    },
    runtime::{eval::evaluate, world::build_canonical_macro_env},
    syntax::parser::wrap_in_do,
    to_error_source, AstNode, MacroRegistry, OutputBuffer, SharedOutput, SutraError, Value, World,
};

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
    /// Executes Sutra source code through the complete pipeline.
    /// This is the single entry point for all execution paths, including tests.
    pub fn execute(&self, source: &str, output: SharedOutput) -> Result<(), SutraError> {
        let src_arc = to_error_source(source);
        // Phase 1: Parse the source into AST nodes
        let ast_nodes = crate::syntax::parser::parse(source)?;

        // Phase 2: Partition AST nodes: macro definitions vs user code
        // Note: define forms are special forms that should be evaluated, not treated as macros
        let (macro_defs, user_code): (Vec<_>, Vec<_>) =
            ast_nodes.into_iter().partition(|_expr| false); // Don't partition define forms as macros

        // Phase 3: Build canonical macro environment
        let mut env = build_canonical_macro_env()?;

        // Phase 4: Extend env.user_macros with user-defined macros parsed from the source.
        for macro_expr in macro_defs {
            let (name, template) = parse_macro_definition(&macro_expr)?;
            if env.user_macros.contains_key(&name) {
                return Err(err_msg!(Validation, "Duplicate macro name '{}'", name));
            }
            env.user_macros
                .insert(name.clone(), MacroDefinition::Template(template));
        }

        // Phase 5: Wrap user_code in a (do ...) if needed
        let program = wrap_in_do(user_code);

        // Phase 6: Expand macros (CRITICAL: This happens BEFORE evaluation)
        let expanded = expand_macros_recursively(program, &mut env)?;

        // Phase 7: Validation step (optional but recommended)
        if self.validate {
            let atom_registry = crate::runtime::world::build_default_atom_registry();
            let mut combined_macros = env.core_macros.clone();
            combined_macros.extend(env.user_macros.clone());
            let macro_registry_for_validation = MacroRegistry {
                macros: combined_macros,
            };
            let validation_result = crate::validation::semantic::validate_expanded_ast(
                &expanded,
                &macro_registry_for_validation,
                &atom_registry,
            );

            if !validation_result.is_valid() {
                let error_message = validation_result.errors.join("\n");
                return Err(err_ctx!(
                    Validation,
                    format!("Semantic validation failed:\n{}", error_message),
                    &src_arc,
                    expanded.span,
                    "Check for undefined symbols, type errors, or invalid macro usage in your code."
                ));
            }
        }

        // Phase 8: Evaluate the expanded AST (CRITICAL: No macro expansion happens here)
        let world = World::default();
        let source = Arc::new(NamedSource::new("source", source.to_string()));
        let atom_registry = crate::runtime::world::build_default_atom_registry();
        let (result, _updated_world) = evaluate(
            &expanded,
            &world,
            output.clone(),
            &atom_registry,
            source.clone(),
            self.max_depth,
        )?;

        // Phase 9: Output result (if not nil)
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

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
        source: Arc<NamedSource<String>>,
    ) -> Result<(Value, World), SutraError> {
        let atom_registry = crate::runtime::world::build_default_atom_registry();
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
    pub fn execute_test(
        &self,
        test_body: &AstNode,
        output: SharedOutput,
    ) -> Result<(), SutraError> {
        // Phase 1: Build canonical macro environment (includes null?, etc.)
        let mut env = build_canonical_macro_env()?;

        // Phase 2: Expand macros in the test body
        let expanded = expand_macros_recursively(test_body.clone(), &mut env)?;

        // Phase 3: Evaluate the expanded AST
        let world = World::default();
        let source = Arc::new(NamedSource::new("test", "".to_string()));
        let atom_registry = crate::runtime::world::build_default_atom_registry();
        let (result, _updated_world) = evaluate(
            &expanded,
            &world,
            output.clone(),
            &atom_registry,
            source.clone(),
            self.max_depth,
        )?;

        // Phase 4: Output result (if not nil)
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    /// Executes AST nodes directly without parsing, avoiding double execution.
    /// This is optimized for test execution where AST is already available.
    pub fn execute_ast(&self, nodes: &[AstNode]) -> Result<Value, SutraError> {
        // Partition AST nodes: macro definitions vs user code
        let (macro_defs, user_code) = nodes.iter().cloned().partition(is_macro_definition);

        // Build canonical macro environment
        let mut env = build_canonical_macro_env()?;

        // Extend env.user_macros with user-defined macros
        for macro_expr in macro_defs {
            let (name, template) = parse_macro_definition(&macro_expr)?;
            if env.user_macros.contains_key(&name) {
                return Err(err_msg!(Validation, "Duplicate macro name '{}'", name));
            }
            env.user_macros
                .insert(name.clone(), MacroDefinition::Template(template));
        }

        // Wrap user_code in a (do ...) if needed
        let program = wrap_in_do(user_code);

        // Expand macros
        let expanded = expand_macros_recursively(program, &mut env)?;

        // Optional validation step
        if self.validate {
            let atom_registry = crate::runtime::world::build_default_atom_registry();
            let mut combined_macros = env.core_macros.clone();
            combined_macros.extend(env.user_macros.clone());
            let macro_registry = MacroRegistry {
                macros: combined_macros,
            };

            let validation_result = crate::validation::semantic::validate_expanded_ast(
                &expanded,
                &macro_registry,
                &atom_registry,
            );

            if !validation_result.is_valid() {
                let errors = validation_result.errors.join("\n");
                let src_arc = to_error_source("");
                return Err(err_ctx!(
                    Validation,
                    format!("Semantic validation failed:\n{}", errors),
                    &src_arc,
                    expanded.span,
                    "Check for undefined symbols, type errors, or invalid macro usage in your code."
                ));
            }
        }

        // Evaluate the expanded AST
        let world = World::default();
        let source = Arc::new(NamedSource::new("ast_execution", "".to_string()));
        let atom_registry = crate::runtime::world::build_default_atom_registry();
        let output = SharedOutput(Rc::new(RefCell::new(OutputBuffer::new())));

        let (result, _) = evaluate(
            &expanded,
            &world,
            output,
            &atom_registry,
            source,
            self.max_depth,
        )?;

        Ok(result)
    }
}
