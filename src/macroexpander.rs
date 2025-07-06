//! # Macroexpander Module
//!
//! ## Purpose
//! Expands macros in the AST, supporting both built-in and user-defined macros. Pure transformation; never mutates input AST. All nodes and errors carry spans.
//!
//! ## Core Principles
//! - Pure, stateless, composable
//! - All data/errors carry spans
//! - Serde-compatible, testable, minimal
//!
//! ## Invariants
//! - Never mutates input
//! - Macro expansion is pure, deterministic, and bounded
//!
//! ## Changelog
//! - 2025-07-05: Initial stub by AI. Rationale: Canonical modular pipeline contract.

use serde::{Serialize, Deserialize};

/// Main trait for the macroexpander stage.
pub trait SutraMacroExpander {
    /// Expands macros in the AST. Returns expanded AST or macro error.
    fn expand_macros(&self, ast: WithSpan<SutraAstNode>, context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraMacroContext {
    // Registry, hygiene scope, etc.
    pub registry: MacroRegistry,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraMacroError {
    pub span: SutraSpan,
    pub macro_name: String,
    pub message: String, // Must start with macro name and describe expected vs. found
}

/// # Example
/// ```rust
/// let expander = MyMacroExpander::default();
/// let expanded = expander.expand_macros(ast, &macro_context)?;
/// ```

//! ## Anti-Patterns Checklist
//! - Never mutate input or global state.
//! - Never expose internal fields.
//! - Never use .unwrap(), .expect(), or panic! in production.
//! - Never use trait objects unless required.
//! - Never use macros for main logic (except error helpers, with justification).

/// Minimal, robust macroexpander for the canonical pipeline.
///
/// Uses the macro registry provided in the context. This ensures that both built-in and user-defined macros
/// registered in the context are available for expansion. Never creates a new registry internally.
pub struct MinimalMacroExpander;

impl SutraMacroExpander for MinimalMacroExpander {
    fn expand_macros(
        &self,
        ast: WithSpan<SutraAstNode>,
        context: &SutraMacroContext,
    ) -> Result<WithSpan<SutraAstNode>, SutraMacroError> {
        expand_macros_recursive(ast, context)
    }
}

/// Recursively expands macros in the AST using the registry.
pub fn expand_macros_recursive(
    node: WithSpan<SutraAstNode>,
    context: &SutraMacroContext,
) -> Result<WithSpan<SutraAstNode>, SutraMacroError> {
    // Guard: Not a list? Return as-is.
    let SutraAstNode::List(items) = node.value else {
        return Ok(WithSpan { value: node.value, span: node.span });
    };

    // Guard: Empty list? Return empty list.
    if items.is_empty() {
        return Ok(WithSpan { value: SutraAstNode::List(vec![]), span: node.span });
    }

    // Guard: First element not a symbol? Recursively expand all children.
    let Some(SutraAstNode::Symbol(ref name, _)) = items.get(0).map(|w| &w.value) else {
        return Ok(WithSpan {
            value: SutraAstNode::List(expand_all(items, context)?),
            span: node.span,
        });
    };

    // Macro dispatch: If handler exists, expand as macro.
    if let Some(handler) = context.registry.get(name) {
        let expanded_args = expand_all(items[1..].to_vec(), context)?;
        return handler.expand(&expanded_args, &node.span, context);
    }

    // Not a macro call: recursively expand all children.
    Ok(WithSpan {
        value: SutraAstNode::List(expand_all(items, context)?),
        span: node.span,
    })
}

/// Helper: Recursively expands a vector of AST nodes.
fn expand_all(
    items: Vec<WithSpan<SutraAstNode>>,
    context: &SutraMacroContext,
) -> Result<Vec<WithSpan<SutraAstNode>>, SutraMacroError> {
    items.into_iter()
        .map(|item| expand_macros_recursive(item, context))
        .collect()
}

/// Trait for all macro handlers (built-in and user-defined).
pub trait MacroHandler: Send + Sync {
    /// Expands a macro call with the given arguments and context.
    fn expand(
        &self,
        args: &[WithSpan<SutraAstNode>],
        span: &SutraSpan,
        context: &SutraMacroContext,
    ) -> Result<WithSpan<SutraAstNode>, SutraMacroError>;
}

/// Registry for macro handlers (built-in and user-defined).
pub struct MacroRegistry {
    handlers: std::collections::HashMap<String, Box<dyn MacroHandler>>,
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self { handlers: std::collections::HashMap::new() }
    }
    /// Registers a macro handler (built-in or user-defined).
    pub fn register(&mut self, name: &str, handler: Box<dyn MacroHandler>) {
        self.handlers.insert(name.to_string(), handler);
    }
    /// Looks up a macro handler by name.
    pub fn get(&self, name: &str) -> Option<&Box<dyn MacroHandler>> {
        self.handlers.get(name)
    }
    /// Registers all built-in macros.
    pub fn register_builtins(&mut self) {
        self.register("quote", Box::new(QuoteMacro));
        self.register("if", Box::new(IfMacro));
    }
}

/// Built-in 'quote' macro handler.
pub struct QuoteMacro;
impl MacroHandler for QuoteMacro {
    fn expand(&self, args: &[WithSpan<SutraAstNode>], span: &SutraSpan, _context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError> {
        if args.len() != 1 {
            return Err(SutraMacroError {
                span: span.clone(),
                macro_name: "quote".to_string(),
                message: "quote: expected exactly one argument".to_string(),
            });
        }
        Ok(WithSpan {
            value: SutraAstNode::Quote(Box::new(args[0].clone()), span.clone()),
            span: span.clone(),
        })
    }
}

/// Built-in 'if' macro handler.
pub struct IfMacro;
impl MacroHandler for IfMacro {
    fn expand(&self, args: &[WithSpan<SutraAstNode>], span: &SutraSpan, _context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError> {
        if args.len() != 3 {
            return Err(SutraMacroError {
                span: span.clone(),
                macro_name: "if".to_string(),
                message: "if: expected exactly three arguments (condition, then, else)".to_string(),
            });
        }
        Ok(WithSpan {
            value: SutraAstNode::If {
                condition: Box::new(args[0].clone()),
                then_branch: Box::new(args[1].clone()),
                else_branch: Box::new(args[2].clone()),
                span: span.clone(),
            },
            span: span.clone(),
        })
    }
}

/// Trait for hygiene strategies (can be extended in the future).
pub trait Hygiene: Send + Sync {
    fn gensym(&self, base: &str) -> String;
}

/// Default hygiene: appends a unique suffix.
pub struct DefaultHygiene;
impl Hygiene for DefaultHygiene {
    fn gensym(&self, base: &str) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{}_gensym_{}", base, id)
    }
}

/// A user-defined macro with a template body, parameters, and optional hygiene.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct UserTemplateMacro {
    pub params: Vec<String>,
    pub body: WithSpan<SutraAstNode>,
    pub doc: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip)]
    pub hygiene: Option<Box<dyn Hygiene>>,
}

impl MacroHandler for UserTemplateMacro {
    fn expand(
        &self,
        args: &[WithSpan<SutraAstNode>],
        span: &SutraSpan,
        context: &SutraMacroContext,
    ) -> Result<WithSpan<SutraAstNode>, SutraMacroError> {
        if args.len() != self.params.len() {
            return Err(SutraMacroError {
                span: span.clone(),
                macro_name: "user_macro".to_string(),
                message: format!(
                    "user macro: expected {} arguments, got {}",
                    self.params.len(),
                    args.len()
                ),
            });
        }
        // Build a substitution environment: param name -> argument AST
        let mut env = std::collections::HashMap::new();
        for (name, arg) in self.params.iter().zip(args.iter()) {
            env.insert(name.clone(), arg.clone());
        }
        // Substitute parameters in the template body (with hygiene stub)
        let substituted = substitute_node(&self.body, &env, self.hygiene.as_deref());
        // Recursively expand the result
        expand_macros_recursive(substituted, context)
    }
}

/// Handles hygiene-specific substitution for let bindings in macro templates.
/// Returns (new_name, new_env, new_binding) where:
/// - new_name: the gensym'd name
/// - new_env: the updated environment with the new binding
/// - new_binding: the WithSpan symbol node for the new binding
fn substitute_let_binding_with_hygiene(
    binding: &WithSpan<SutraAstNode>,
    env: &std::collections::HashMap<String, WithSpan<SutraAstNode>>,
    hygiene: &dyn Hygiene,
) -> (String, std::collections::HashMap<String, WithSpan<SutraAstNode>>, WithSpan<SutraAstNode>) {
    if let SutraAstNode::Symbol(ref bind_name, _) = binding.value {
        let new_name = hygiene.gensym(bind_name);
        let mut new_env = env.clone();
        new_env.insert(
            bind_name.clone(),
            WithSpan {
                value: SutraAstNode::Symbol(new_name.clone(), binding.span.clone()),
                span: binding.span.clone(),
            },
        );
        let new_binding = WithSpan {
            value: SutraAstNode::Symbol(new_name.clone(), binding.span.clone()),
            span: binding.span.clone(),
        };
        (new_name, new_env, new_binding)
    } else {
        // Fallback: return original name and env if not a symbol
        (String::new(), env.clone(), binding.clone())
    }
}

/// Substitute a symbol node using the environment.
fn substitute_symbol(
    name: &str,
    span: &SutraSpan,
    env: &std::collections::HashMap<String, WithSpan<SutraAstNode>>,
) -> WithSpan<SutraAstNode> {
    env.get(name)
        .cloned()
        .unwrap_or(WithSpan {
            value: SutraAstNode::Symbol(name.to_string(), span.clone()),
            span: span.clone(),
        })
}

/// Substitute a list node, handling hygiene for 'let' forms if needed.
fn substitute_list(
    items: &[WithSpan<SutraAstNode>],
    span: &SutraSpan,
    env: &std::collections::HashMap<String, WithSpan<SutraAstNode>>,
    hygiene: Option<&dyn Hygiene>,
) -> WithSpan<SutraAstNode> {
    // Guard: must be a 'let' form
    let Some(WithSpan { value: SutraAstNode::Symbol(let_kw, _), .. }) = items.get(0) else {
        return WithSpan {
            value: SutraAstNode::List(
                items.iter().map(|item| substitute_node(item, env, hygiene)).collect(),
            ),
            span: span.clone(),
        };
    };
    // Guard: must have hygiene
    let Some(h) = hygiene else {
        return WithSpan {
            value: SutraAstNode::List(
                items.iter().map(|item| substitute_node(item, env, hygiene)).collect(),
            ),
            span: span.clone(),
        };
    };
    // Guard: must have a binding
    let Some(binding) = items.get(1) else {
        return WithSpan {
            value: SutraAstNode::List(
                items.iter().map(|item| substitute_node(item, env, hygiene)).collect(),
            ),
            span: span.clone(),
        };
    };
    // All guards passed: do hygiene logic and return
    let (new_name, new_env, new_binding) = substitute_let_binding_with_hygiene(binding, env, h);
    let new_body: Vec<WithSpan<SutraAstNode>> = items[2..]
        .iter()
        .map(|item| substitute_node(item, &new_env, hygiene))
        .collect();
    WithSpan {
        value: SutraAstNode::List(
            vec![items[0].clone(), new_binding]
                .into_iter()
                .chain(new_body)
                .collect(),
        ),
        span: span.clone(),
    }
}

/// Substitute parameters in the template AST with arguments from the environment.
/// Dispatches to helpers by node type. Applies hygiene if provided.
fn substitute_node(
    node: &WithSpan<SutraAstNode>,
    env: &std::collections::HashMap<String, WithSpan<SutraAstNode>>,
    hygiene: Option<&dyn Hygiene>,
) -> WithSpan<SutraAstNode> {
    match &node.value {
        SutraAstNode::Symbol(name, _) => substitute_symbol(name, &node.span, env),
        SutraAstNode::List(items) => substitute_list(items, &node.span, env, hygiene),
        _ => node.clone(),
    }
}

#[cfg(test)]
mod hygiene_tests {
    use super::*;
    use crate::ast_builder::{WithSpan, SutraAstNode};
    use crate::cst_parser::SutraSpan;

    struct TestHygiene;
    impl Hygiene for TestHygiene {
        fn gensym(&self, base: &str) -> String {
            format!("{}_unique", base)
        }
    }

    #[test]
    fn user_macro_with_let_binding_is_hygienic() {
        // Macro: (user_let x) => (let tmp x (do-something tmp))
        let user_macro = UserTemplateMacro {
            params: vec!["x".to_string()],
            body: WithSpan {
                value: SutraAstNode::List(vec![
                    WithSpan {
                        value: SutraAstNode::Symbol("let".to_string(), SutraSpan { start: 0, end: 3 }),
                        span: SutraSpan { start: 0, end: 3 },
                    },
                    WithSpan {
                        value: SutraAstNode::Symbol("tmp".to_string(), SutraSpan { start: 4, end: 7 }),
                        span: SutraSpan { start: 4, end: 7 },
                    },
                    WithSpan {
                        value: SutraAstNode::Symbol("x".to_string(), SutraSpan { start: 8, end: 9 }),
                        span: SutraSpan { start: 8, end: 9 },
                    },
                    WithSpan {
                        value: SutraAstNode::List(vec![
                            WithSpan {
                                value: SutraAstNode::Symbol("do-something".to_string(), SutraSpan { start: 10, end: 22 }),
                                span: SutraSpan { start: 10, end: 22 },
                            },
                            WithSpan {
                                value: SutraAstNode::Symbol("tmp".to_string(), SutraSpan { start: 23, end: 26 }),
                                span: SutraSpan { start: 23, end: 26 },
                            },
                        ]),
                        span: SutraSpan { start: 10, end: 26 },
                    },
                ]),
                span: SutraSpan { start: 0, end: 26 },
            },
            doc: Some("A user macro with a hygienic let binding.".to_string()),
            metadata: None,
            hygiene: Some(Box::new(TestHygiene)),
        };
        let mut reg = MacroRegistry::new();
        reg.register_builtins();
        reg.register("user_let", Box::new(user_macro));
        let context = SutraMacroContext { registry: reg };
        let arg = WithSpan {
            value: SutraAstNode::Symbol("foo".to_string(), SutraSpan { start: 30, end: 33 }),
            span: SutraSpan { start: 30, end: 33 },
        };
        let ast = WithSpan {
            value: SutraAstNode::List(vec![
                WithSpan {
                    value: SutraAstNode::Symbol("user_let".to_string(), SutraSpan { start: 0, end: 8 }),
                    span: SutraSpan { start: 0, end: 8 },
                },
                arg.clone(),
            ]),
            span: SutraSpan { start: 0, end: 33 },
        };
        let expander = MinimalMacroExpander;
        let expanded = expander.expand_macros(ast, &context).unwrap();
        match expanded.value {
            SutraAstNode::List(ref items) => {
                // Should be (let tmp_unique foo (do-something tmp_unique))
                assert_eq!(items.len(), 4);
                assert_eq!(items[0].value, SutraAstNode::Symbol("let".to_string(), SutraSpan { start: 0, end: 3 }));
                assert_eq!(items[1].value, SutraAstNode::Symbol("tmp_unique".to_string(), SutraSpan { start: 4, end: 7 }));
                assert_eq!(items[2].value, arg.value);
                match &items[3].value {
                    SutraAstNode::List(inner) => {
                        assert_eq!(inner[0].value, SutraAstNode::Symbol("do-something".to_string(), SutraSpan { start: 10, end: 22 }));
                        assert_eq!(inner[1].value, SutraAstNode::Symbol("tmp_unique".to_string(), SutraSpan { start: 23, end: 26 }));
                    }
                    _ => panic!("Expected inner list"),
                }
            }
            _ => panic!("Expected List node"),
        }
    }

    #[test]
    fn user_macro_wrong_argument_count_returns_error() {
        let user_macro = UserTemplateMacro {
            params: vec!["x".to_string()],
            body: WithSpan {
                value: SutraAstNode::Symbol("x".to_string(), SutraSpan { start: 0, end: 1 }),
                span: SutraSpan { start: 0, end: 1 },
            },
            doc: None,
            metadata: None,
            hygiene: None,
        };
        let mut reg = MacroRegistry::new();
        reg.register("user_macro", Box::new(user_macro));
        let context = SutraMacroContext { registry: reg };
        let ast = WithSpan {
            value: SutraAstNode::List(vec![
                WithSpan {
                    value: SutraAstNode::Symbol("user_macro".to_string(), SutraSpan { start: 0, end: 10 }),
                    span: SutraSpan { start: 0, end: 10 },
                },
                WithSpan {
                    value: SutraAstNode::Symbol("foo".to_string(), SutraSpan { start: 11, end: 14 }),
                    span: SutraSpan { start: 11, end: 14 },
                },
                WithSpan {
                    value: SutraAstNode::Symbol("bar".to_string(), SutraSpan { start: 15, end: 18 }),
                    span: SutraSpan { start: 15, end: 18 },
                },
            ]),
            span: SutraSpan { start: 0, end: 18 },
        };
        let expander = MinimalMacroExpander;
        let result = expander.expand_macros(ast, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("expected 1 arguments, got 2"));
    }

    #[test]
    fn missing_macro_handler_returns_unexpanded_list() {
        let reg = MacroRegistry::new();
        let context = SutraMacroContext { registry: reg };
        let ast = WithSpan {
            value: SutraAstNode::List(vec![
                WithSpan {
                    value: SutraAstNode::Symbol("nonexistent".to_string(), SutraSpan { start: 0, end: 10 }),
                    span: SutraSpan { start: 0, end: 10 },
                },
                WithSpan {
                    value: SutraAstNode::Symbol("foo".to_string(), SutraSpan { start: 11, end: 14 }),
                    span: SutraSpan { start: 11, end: 14 },
                },
            ]),
            span: SutraSpan { start: 0, end: 14 },
        };
        let expander = MinimalMacroExpander;
        let result = expander.expand_macros(ast.clone(), &context);
        assert!(result.is_ok());
        let expanded = result.unwrap();
        // Should be unchanged
        assert_eq!(expanded.value, ast.value);
    }

    #[test]
    fn user_macro_with_non_symbol_let_binding_falls_back() {
        struct DummyHygiene;
        impl Hygiene for DummyHygiene {
            fn gensym(&self, base: &str) -> String {
                format!("{}_dummy", base)
            }
        }
        let user_macro = UserTemplateMacro {
            params: vec!["x".to_string()],
            body: WithSpan {
                value: SutraAstNode::List(vec![
                    WithSpan {
                        value: SutraAstNode::Symbol("let".to_string(), SutraSpan { start: 0, end: 3 }),
                        span: SutraSpan { start: 0, end: 3 },
                    },
                    WithSpan {
                        value: SutraAstNode::List(vec![]), // Not a symbol
                        span: SutraSpan { start: 4, end: 5 },
                    },
                    WithSpan {
                        value: SutraAstNode::Symbol("x".to_string(), SutraSpan { start: 6, end: 7 }),
                        span: SutraSpan { start: 6, end: 7 },
                    },
                ]),
                span: SutraSpan { start: 0, end: 7 },
            },
            doc: None,
            metadata: None,
            hygiene: Some(Box::new(DummyHygiene)),
        };
        let mut reg = MacroRegistry::new();
        reg.register("user_let", Box::new(user_macro));
        let context = SutraMacroContext { registry: reg };
        let arg = WithSpan {
            value: SutraAstNode::Symbol("foo".to_string(), SutraSpan { start: 10, end: 13 }),
            span: SutraSpan { start: 10, end: 13 },
        };
        let ast = WithSpan {
            value: SutraAstNode::List(vec![
                WithSpan {
                    value: SutraAstNode::Symbol("user_let".to_string(), SutraSpan { start: 0, end: 8 }),
                    span: SutraSpan { start: 0, end: 8 },
                },
                arg.clone(),
            ]),
            span: SutraSpan { start: 0, end: 13 },
        };
        let expander = MinimalMacroExpander;
        let expanded = expander.expand_macros(ast, &context).unwrap();
        // Should not panic or substitute, just use the non-symbol as binding
        match expanded.value {
            SutraAstNode::List(ref items) => {
                assert_eq!(items[0].value, SutraAstNode::Symbol("let".to_string(), SutraSpan { start: 0, end: 3 }));
                assert!(matches!(items[1].value, SutraAstNode::List(_))); // Non-symbol binding
            }
            _ => panic!("Expected List node"),
        }
    }
}