//! # Clean Macro Expansion System
//!
//! Core responsibility: Transform user-friendly syntax into canonical AST forms.
//!
//! This module performs three essential functions:
//! 1. Template substitution - Replace parameters in macro bodies with arguments
//! 2. Path canonicalization - Convert various path syntaxes to Expr::Path nodes
//! 3. Built-in macro expansion - Transform syntax sugar into canonical forms

use miette::NamedSource;
use std::collections::HashMap;
use std::sync::Arc;

use crate::prelude::*;
use crate::{
    errors::{to_source_span, ErrorKind, ErrorReporting, SourceContext, SutraError},
    syntax::{parser, ParamList},
    validation::ValidationContext,
};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum recursion depth for macro expansion to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 100;

/// Expected arity for single-argument macros
const SINGLE_ARG_ARITY: usize = 1;

/// Expected arity for two-argument macros
const DUAL_ARG_ARITY: usize = 2;

/// Expected arity for macro definitions (define name params body)
const MACRO_DEFINITION_ARITY: usize = 3;

// ============================================================================
// CORE TYPES
// ============================================================================

/// A native Rust function that transforms an AST node
pub type MacroFunction = fn(&AstNode) -> Result<AstNode, SutraError>;

/// A template macro (for compatibility)
#[derive(Debug, Clone, PartialEq)]
pub struct MacroTemplate {
    pub params: ParamList,
    pub body: Box<AstNode>,
}

impl MacroTemplate {
    pub fn new(params: ParamList, body: Box<AstNode>) -> Result<Self, SutraError> {
        Ok(MacroTemplate { params, body })
    }
}

/// A macro definition - either a native function or template
#[derive(Debug, Clone)]
pub enum MacroDefinition {
    /// Native Rust function macro
    Function(MacroFunction),
    /// A declarative template macro (compatibility tuple variant)
    Template(MacroTemplate),
}

/// Origin of a macro expansion step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroOrigin {
    User,
    Core,
}

/// The complete macro expansion system
#[derive(Debug, Clone)]
pub struct MacroSystem {
    macros: HashMap<String, MacroDefinition>,
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

impl MacroSystem {
    /// Create a new macro system with built-in macros registered
    pub fn new(_source: Arc<NamedSource<String>>) -> Self {
        let mut system = Self {
            macros: HashMap::new(),
        };
        system.register_builtins();
        system
    }

    /// Expand all macros in an AST node
    pub fn expand(&self, ast: AstNode) -> Result<AstNode, SutraError> {
        self.expand_recursive(ast, 0)
    }

    /// Load and register macros from source code
    pub fn load_from_source(&mut self, source: &str) -> Result<(), SutraError> {
        let source_ctx = SourceContext::from_file("macro_source", source);
        let exprs = parser::parse(source, source_ctx)?;

        for expr in exprs {
            if let Some((name, def)) = self.parse_macro_definition(&expr)? {
                self.macros.insert(name, def);
            }
        }
        Ok(())
    }

    /// Register a macro (used for built-ins and user macros)
    pub fn register(&mut self, name: String, definition: MacroDefinition) {
        self.macros.insert(name, definition);
    }

    /// Register a user macro (compatibility method)
    pub fn register_user_macro(
        &mut self,
        name: String,
        definition: MacroDefinition,
        _allow_overwrite: bool,
    ) -> Result<(), SutraError> {
        self.register(name, definition);
        Ok(())
    }

    /// Look up a macro by name (compatibility method)
    pub fn lookup_macro(&self, name: &str) -> Option<(MacroOrigin, &MacroDefinition)> {
        self.macros.get(name).map(|def| (MacroOrigin::User, def))
    }

    /// Get all macro names (compatibility method)
    pub fn macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }

    // ========================================================================
    // CORE EXPANSION LOGIC
    // ========================================================================

    /// Recursively expand macros with depth checking
    fn expand_recursive(&self, node: AstNode, depth: usize) -> Result<AstNode, SutraError> {
        if depth > MAX_RECURSION_DEPTH {
            let ctx = ValidationContext::new(
                SourceContext::from_file("macro_expansion", "recursion_limit"),
                "macro_expansion".to_string(),
            );
            return Err(ctx.report(ErrorKind::RecursionLimit, to_source_span(node.span)));
        }

        // Check if this is a macro call
        let Expr::List(items, _) = &*node.value else {
            return self.expand_subforms(node, depth);
        };

        let Some(first) = items.first() else {
            return self.expand_subforms(node, depth);
        };

        let Expr::Symbol(name, _) = &*first.value else {
            return self.expand_subforms(node, depth);
        };

        if let Some(macro_def) = self.macros.get(name) {
            let expanded = self.apply_macro(&node, macro_def)?;
            return self.expand_recursive(expanded, depth + 1);
        }

        self.expand_subforms(node, depth)
    }

    /// Helper to expand subforms without macro call detection
    fn expand_subforms(&self, node: AstNode, depth: usize) -> Result<AstNode, SutraError> {
        match &*node.value {
            Expr::List(items, span) => {
                let mut expanded_items = Vec::new();
                for item in items {
                    expanded_items.push(self.expand_recursive(item.clone(), depth)?);
                }
                Ok(Spanned {
                    value: Expr::List(expanded_items, *span).into(),
                    span: node.span,
                })
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let new_condition = self.expand_recursive(condition.as_ref().clone(), depth)?;
                let new_then = self.expand_recursive(then_branch.as_ref().clone(), depth)?;
                let new_else = self.expand_recursive(else_branch.as_ref().clone(), depth)?;
                Ok(Spanned {
                    value: Expr::If {
                        condition: Box::new(new_condition),
                        then_branch: Box::new(new_then),
                        else_branch: Box::new(new_else),
                        span: *span,
                    }
                    .into(),
                    span: node.span,
                })
            }
            Expr::Quote(inner, span) => {
                let new_inner = self.expand_recursive(inner.as_ref().clone(), depth)?;
                Ok(Spanned {
                    value: Expr::Quote(Box::new(new_inner), *span).into(),
                    span: node.span,
                })
            }
            Expr::Spread(inner) => {
                let new_inner = self.expand_recursive(inner.as_ref().clone(), depth)?;
                Ok(Spanned {
                    value: Expr::Spread(Box::new(new_inner)).into(),
                    span: node.span,
                })
            }
            _ => Ok(node),
        }
    }

    /// Apply a single macro to a call node
    fn apply_macro(
        &self,
        call: &AstNode,
        definition: &MacroDefinition,
    ) -> Result<AstNode, SutraError> {
        match definition {
            MacroDefinition::Function(f) => f(call),
            MacroDefinition::Template(template) => {
                let (args, _span) = self.extract_args(call)?;

                // Validate arity
                let required_count = template.params.required.len();
                let has_variadic = template.params.rest.is_some();

                if args.len() < required_count || (args.len() > required_count && !has_variadic) {
                    let ctx = ValidationContext::new(
                        SourceContext::from_file("macro_expansion", "arity_check"),
                        "macro_expansion".to_string(),
                    );
                    let expected = if has_variadic {
                        format!("at least {}", required_count)
                    } else {
                        required_count.to_string()
                    };
                    return Err(ctx.arity_mismatch(
                        &expected,
                        args.len(),
                        to_source_span(call.span),
                    ));
                }

                // Bind parameters
                let mut bindings = HashMap::new();
                for (i, param) in template.params.required.iter().enumerate() {
                    bindings.insert(param.clone(), args[i].clone());
                }

                // Handle variadic parameter
                if let Some(var_name) = &template.params.rest {
                    let rest_args: Vec<AstNode> = if args.len() > required_count {
                        args[required_count..].to_vec()
                    } else {
                        Vec::new()
                    };

                    // Create (list ...rest_args)
                    let mut list_items = vec![Spanned {
                        value: Expr::Symbol("list".to_string(), call.span).into(),
                        span: call.span,
                    }];
                    list_items.extend(rest_args);

                    let list_node = Spanned {
                        value: Expr::List(list_items, call.span).into(),
                        span: call.span,
                    };
                    bindings.insert(var_name.clone(), list_node);
                }

                self.substitute(&template.body, &bindings)
            }
        }
    }

    /// Substitute parameters in a template body
    fn substitute(
        &self,
        node: &AstNode,
        bindings: &HashMap<String, AstNode>,
    ) -> Result<AstNode, SutraError> {
        match &*node.value {
            Expr::Symbol(name, _) => {
                Ok(bindings.get(name).cloned().unwrap_or_else(|| node.clone()))
            }
            Expr::List(items, span) => {
                let mut new_items = Vec::new();
                for item in items {
                    if let Expr::Spread(inner) = &*item.value {
                        let substituted = self.substitute(inner, bindings)?;
                        if let Expr::List(elements, _) = &*substituted.value {
                            new_items.extend(elements.clone());
                        } else {
                            let ctx = ValidationContext::new(
                                SourceContext::from_file("macro_expansion", "spread_error"),
                                "macro_expansion".to_string(),
                            );
                            return Err(ctx.type_mismatch(
                                "list",
                                substituted.value.type_name(),
                                to_source_span(inner.span),
                            ));
                        }
                    } else {
                        new_items.push(self.substitute(item, bindings)?);
                    }
                }
                Ok(Spanned {
                    value: Expr::List(new_items, *span).into(),
                    span: node.span,
                })
            }
            _ => Ok(node.clone()),
        }
    }

    // ========================================================================
    // PARSING AND UTILITIES
    // ========================================================================

    /// Parse a macro definition from AST
    fn parse_macro_definition(
        &self,
        expr: &AstNode,
    ) -> Result<Option<(String, MacroDefinition)>, SutraError> {
        let Expr::List(items, _) = &*expr.value else {
            return Ok(None);
        };

        if items.len() != MACRO_DEFINITION_ARITY {
            return Ok(None);
        }

        let Expr::Symbol(def_name, _) = &*items[0].value else {
            return Ok(None);
        };
        if def_name != "define" {
            return Ok(None);
        }

        let Expr::ParamList(param_list) = &*items[1].value else {
            return Ok(None);
        };

        if param_list.required.is_empty() {
            let ctx = ValidationContext::new(
                SourceContext::from_file("macro_definition", "empty_params"),
                "macro_definition".to_string(),
            );
            return Err(ctx.report(
                ErrorKind::MissingElement {
                    element: "macro name".to_string(),
                },
                to_source_span(items[1].span),
            ));
        }

        let macro_name = param_list.required[0].clone();
        let params = param_list.required[1..].to_vec();
        let variadic = param_list.rest.clone();
        let body = items[2].clone();

        let template = MacroTemplate {
            params: ParamList {
                required: params,
                rest: variadic,
                span: param_list.span,
            },
            body: Box::new(body),
        };

        Ok(Some((macro_name, MacroDefinition::Template(template))))
    }

    /// Extract arguments from a macro call
    fn extract_args(&self, call: &AstNode) -> Result<(Vec<AstNode>, Span), SutraError> {
        Self::extract_args_from_call(call)
    }

    /// Static helper to extract arguments from macro calls
    fn extract_args_from_call(call: &AstNode) -> Result<(Vec<AstNode>, Span), SutraError> {
        let Expr::List(items, span) = &*call.value else {
            let ctx = ValidationContext::new(
                SourceContext::from_file("macro_expansion", "invalid_call"),
                "macro_expansion".to_string(),
            );
            return Err(ctx.report(
                ErrorKind::MalformedConstruct {
                    construct: "macro call".to_string(),
                },
                to_source_span(call.span),
            ));
        };

        if items.is_empty() {
            let ctx = ValidationContext::new(
                SourceContext::from_file("macro_expansion", "empty_call"),
                "macro_expansion".to_string(),
            );
            return Err(ctx.report(ErrorKind::EmptyExpression, to_source_span(*span)));
        }

        Ok((items[1..].to_vec(), *span))
    }

    // ========================================================================
    // PATH CANONICALIZATION
    // ========================================================================

    /// Static helper to convert expressions to canonical paths
    fn canonical_path_from_expr(expr: &AstNode) -> Result<AstNode, SutraError> {
        let path = match &*expr.value {
            Expr::Symbol(s, _) if s.contains('.') => Path(s.split('.').map(String::from).collect()),
            Expr::Symbol(s, _) => Path(vec![s.clone()]),
            Expr::Path(p, _) => p.clone(),
            Expr::List(items, _) => {
                let mut parts = Vec::new();
                for item in items {
                    match &*item.value {
                        Expr::Symbol(s, _) | Expr::String(s, _) => parts.push(s.clone()),
                        _ => {
                            let ctx = ValidationContext::new(
                                SourceContext::from_file("path_conversion", "invalid_element"),
                                "path_conversion".to_string(),
                            );
                            return Err(ctx.report(
                                ErrorKind::InvalidOperation {
                                    operation: "path_conversion".to_string(),
                                    operand_type: "Path elements must be symbols or strings"
                                        .to_string(),
                                },
                                to_source_span(expr.span),
                            ));
                        }
                    }
                }
                Path(parts)
            }
            _ => {
                let ctx = ValidationContext::new(
                    SourceContext::from_file("path_conversion", "unsupported_type"),
                    "path_conversion".to_string(),
                );
                return Err(ctx.report(
                    ErrorKind::InvalidOperation {
                        operation: "path_conversion".to_string(),
                        operand_type: "Expression cannot be converted to a path".to_string(),
                    },
                    to_source_span(expr.span),
                ));
            }
        };

        Ok(Spanned {
            value: Expr::Path(path, expr.span).into(),
            span: expr.span,
        })
    }

    // ========================================================================
    // BUILT-IN MACROS
    // ========================================================================

    /// Register all built-in macros
    fn register_builtins(&mut self) {
        self.register(
            "set!".to_string(),
            MacroDefinition::Function(Self::expand_set),
        );
        self.register(
            "get".to_string(),
            MacroDefinition::Function(Self::expand_get),
        );
        self.register(
            "del!".to_string(),
            MacroDefinition::Function(Self::expand_del),
        );
        self.register(
            "exists?".to_string(),
            MacroDefinition::Function(Self::expand_exists),
        );
        self.register(
            "inc!".to_string(),
            MacroDefinition::Function(Self::expand_inc),
        );
        self.register(
            "dec!".to_string(),
            MacroDefinition::Function(Self::expand_dec),
        );
        self.register(
            "add!".to_string(),
            MacroDefinition::Function(Self::expand_add),
        );
        self.register(
            "sub!".to_string(),
            MacroDefinition::Function(Self::expand_sub),
        );
        self.register(
            "print".to_string(),
            MacroDefinition::Function(Self::expand_print),
        );
    }

    // ========================================================================
    // BUILT-IN MACRO HELPERS
    // ========================================================================

    /// Validate arity and create error if mismatch
    fn validate_arity(
        args: &[AstNode],
        expected: usize,
        macro_name: &str,
        span: Span,
    ) -> Result<(), SutraError> {
        if args.len() != expected {
            let ctx = ValidationContext::new(
                SourceContext::from_file(macro_name, "arity"),
                "macro_expansion".to_string(),
            );
            return Err(ctx.arity_mismatch(
                &expected.to_string(),
                args.len(),
                to_source_span(span),
            ));
        }
        Ok(())
    }

    /// Create a simple core function call: (core/function_name canonical_path ...)
    fn create_core_call(
        function_name: &str,
        path: AstNode,
        extra_args: &[AstNode],
        span: Span,
    ) -> AstNode {
        let mut items = vec![
            Spanned {
                value: Expr::Symbol(format!("core/{}", function_name), span).into(),
                span,
            },
            path,
        ];
        items.extend(extra_args.iter().cloned());

        Spanned {
            value: Expr::List(items, span).into(),
            span,
        }
    }

    /// Create arithmetic update: (core/set! path (operator (core/get path) value))
    fn create_arithmetic_update(
        path: AstNode,
        operator: &str,
        value: AstNode,
        span: Span,
    ) -> AstNode {
        let get_call = Self::create_core_call("get", path.clone(), &[], span);
        let arithmetic_expr = Spanned {
            value: Expr::List(
                vec![
                    Spanned {
                        value: Expr::Symbol(operator.to_string(), span).into(),
                        span,
                    },
                    get_call,
                    value,
                ],
                span,
            )
            .into(),
            span,
        };
        Self::create_core_call("set!", path, &[arithmetic_expr], span)
    }

    /// Expand (set! path value) to (core/set! canonical_path value)
    fn expand_set(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, DUAL_ARG_ARITY, "set!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_core_call(
            "set!",
            canonical_path,
            &[args[1].clone()],
            span,
        ))
    }

    /// Expand (get path) to (core/get canonical_path)
    fn expand_get(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, SINGLE_ARG_ARITY, "get", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_core_call("get", canonical_path, &[], span))
    }

    /// Expand (del! path) to (core/del! canonical_path)
    fn expand_del(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, SINGLE_ARG_ARITY, "del!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_core_call("del!", canonical_path, &[], span))
    }

    /// Expand (exists? path) to (core/exists? canonical_path)
    fn expand_exists(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, SINGLE_ARG_ARITY, "exists?", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_core_call("exists?", canonical_path, &[], span))
    }

    /// Expand (inc! path) to (core/set! path (+ (core/get path) 1))
    fn expand_inc(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, SINGLE_ARG_ARITY, "inc!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        let one = Spanned {
            value: Expr::Number(1.0, span).into(),
            span,
        };
        Ok(Self::create_arithmetic_update(
            canonical_path,
            "+",
            one,
            span,
        ))
    }

    /// Expand (dec! path) to (core/set! path (- (core/get path) 1))
    fn expand_dec(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, SINGLE_ARG_ARITY, "dec!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        let one = Spanned {
            value: Expr::Number(1.0, span).into(),
            span,
        };
        Ok(Self::create_arithmetic_update(
            canonical_path,
            "-",
            one,
            span,
        ))
    }

    /// Expand (add! path value) to (core/set! path (+ (core/get path) value))
    fn expand_add(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, DUAL_ARG_ARITY, "add!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_arithmetic_update(
            canonical_path,
            "+",
            args[1].clone(),
            span,
        ))
    }

    /// Expand (sub! path value) to (core/set! path (- (core/get path) value))
    fn expand_sub(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;
        Self::validate_arity(&args, DUAL_ARG_ARITY, "sub!", span)?;
        let canonical_path = Self::canonical_path_from_expr(&args[0])?;
        Ok(Self::create_arithmetic_update(
            canonical_path,
            "-",
            args[1].clone(),
            span,
        ))
    }

    /// Expand (print ...) to (core/print ...)
    fn expand_print(call: &AstNode) -> Result<AstNode, SutraError> {
        let (args, span) = Self::extract_args_from_call(call)?;

        let mut items = vec![Spanned {
            value: Expr::Symbol("core/print".to_string(), span).into(),
            span,
        }];
        items.extend(args);

        Ok(Spanned {
            value: Expr::List(items, span).into(),
            span,
        })
    }
}

// ============================================================================
// PUBLIC API FOR COMPATIBILITY
// ============================================================================

/// Create a new macro system (for compatibility)
pub fn create_macro_env(source: Arc<NamedSource<String>>) -> MacroSystem {
    MacroSystem::new(source)
}

/// Expand macros in an AST (for compatibility)
pub fn expand_macros(ast: AstNode, env: &mut MacroSystem) -> Result<AstNode, SutraError> {
    env.expand(ast)
}

/// Load macros from source (for compatibility)
pub fn load_macros_from_source(source: &str, env: &mut MacroSystem) -> Result<(), SutraError> {
    env.load_from_source(source)
}

// ============================================================================
// PUBLIC COMPATIBILITY FUNCTIONS
// ============================================================================

/// Parse a macro definition from AST (public wrapper)
pub fn parse_macro_definition(expr: &AstNode) -> Result<(String, MacroTemplate), SutraError> {
    // Input validation first
    let Expr::List(items, _) = &*expr.value else {
        return Err(create_parse_error(
            "not_macro",
            "macro definition",
            expr.span,
        ));
    };

    if items.len() != MACRO_DEFINITION_ARITY {
        return Err(create_parse_error(
            "not_macro",
            "macro definition",
            expr.span,
        ));
    }

    let Expr::Symbol(def_name, _) = &*items[0].value else {
        return Err(create_parse_error(
            "not_macro",
            "macro definition",
            expr.span,
        ));
    };

    if def_name != "define" {
        return Err(create_parse_error(
            "not_macro",
            "macro definition",
            expr.span,
        ));
    }

    let system = MacroSystem::new(SourceContext::fallback("temp").to_named_source());
    match system.parse_macro_definition(expr)? {
        Some((name, MacroDefinition::Template(template))) => Ok((name, template)),
        Some(_) => {
            let ctx = ValidationContext::new(
                SourceContext::from_file("parse_macro", "function_macro"),
                "macro_parsing".to_string(),
            );
            Err(ctx.report(
                ErrorKind::InvalidOperation {
                    operation: "parse_macro_definition".to_string(),
                    operand_type: "Function macros cannot be parsed from source".to_string(),
                },
                to_source_span(expr.span),
            ))
        }
        None => Err(create_parse_error(
            "not_macro",
            "macro definition",
            expr.span,
        )),
    }
}

/// Helper to create consistent parse errors
fn create_parse_error(context: &str, construct: &str, span: Span) -> SutraError {
    let ctx = ValidationContext::new(
        SourceContext::from_file("parse_macro", context),
        "macro_parsing".to_string(),
    );
    ctx.report(
        ErrorKind::MalformedConstruct {
            construct: construct.to_string(),
        },
        to_source_span(span),
    )
}

/// Expand macros recursively (compatibility alias)
pub fn expand_macros_recursively(
    ast: AstNode,
    env: &mut MacroSystem,
) -> Result<AstNode, SutraError> {
    env.expand(ast)
}

// Note: Removed unnecessary type aliases that just renamed the same types
// Use MacroSystem and Result<AstNode, SutraError> directly instead

// Re-exports for std_macros module (kept for external macro file loading)
pub mod std_macros {
    pub fn register_std_macros(_env: &mut super::MacroSystem) {
        // Built-ins are now registered automatically in MacroSystem::new()
        // This function is kept for compatibility but does nothing
    }
}
