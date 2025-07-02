//! # Sutra Macro Expansion System
//!
//! This module is responsible for the purely syntactic transformation of the AST
//! before evaluation. Macros allow authors to create high-level abstractions
//! that expand into simpler, core expressions.
//!
//! ## Core Principles
//!
//! - **Syntactic Only**: Macros operate solely on the AST (`Expr`). They have no access
//!   to the `World` state and cannot perform any evaluation or side effects.
//! - **Pure Transformation**: Macro expansion is a pure function: `(AST) -> Result<AST, Error>`.
//! - **Inspectable**: The expansion process can be traced, allowing authors to see
//!   how their high-level forms are desugared into core language constructs.
//! - **Layered**: The macro system is a distinct pipeline stage that runs after parsing
//!   and before validation and evaluation.

use crate::ast::Expr;
use crate::error::{SutraError, SutraErrorKind};
use crate::macros_std;
use std::collections::HashMap;

/// Represents a single step in the macro expansion trace.
#[derive(Debug, Clone)]
pub struct TraceStep {
    /// A description of what happened in this step, e.g., "Expanding macro 'is?'".
    pub description: String,
    /// The state of the AST after this step's transformation.
    pub ast: Expr,
}

// A macro function is a native Rust function that transforms an AST.
pub type MacroFn = fn(&Expr) -> Result<Expr, SutraError>;

/// A declarative macro defined by a template.
#[derive(Debug, Clone)]
pub struct MacroTemplate {
    pub params: Vec<String>,
    pub variadic_param: Option<String>,
    pub body: Box<Expr>,
}

/// An enum representing the two types of macros in the system.
#[derive(Debug, Clone)]
pub enum MacroDef {
    Fn(MacroFn),
    Template(MacroTemplate),
}

/// A registry for all known macros, both built-in and potentially user-defined.
///
/// The registry is responsible for dispatching to the correct macro function
/// and for driving the recursive expansion process.
#[derive(Default)]
pub struct MacroRegistry {
    pub macros: HashMap<String, MacroDef>,
}

/// The main entry point for the macro expansion pipeline stage.
/// It creates a standard registry, expands the given expression, and returns the result.
pub fn expand(expr: &Expr) -> Result<Expr, SutraError> {
    let mut registry = MacroRegistry::new();
    // Centralized registration of all standard macros.
    macros_std::register_std_macros(&mut registry);

    // We start at depth 0. The expand_recursive function will handle incrementing it.
    registry.expand_recursive(expr, 0)
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new macro with the given name.
    pub fn register(&mut self, name: &str, func: MacroFn) {
        self.macros.insert(name.to_string(), MacroDef::Fn(func));
    }

    /// Recursively expands all macros in a given expression.
    ///
    /// This is the main entry point for the macro expansion pipeline stage.
    /// It traverses the AST and applies macro transformations wherever a macro
    /// invocation is found.
    ///
    /// `depth` is used to prevent infinite recursion.
    pub fn expand_recursive(&self, expr: &Expr, depth: usize) -> Result<Expr, SutraError> {
        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            return Err(SutraError {
                kind: SutraErrorKind::Macro("Macro expansion depth limit exceeded.".to_string()),
                span: Some(expr.span()),
            });
        }

        // First, handle the case of an `if` expression, which is a special form.
        // We need to recursively expand its branches.
        if let Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } = expr
        {
            let expanded_condition = self.expand_recursive(condition, depth + 1)?;
            let expanded_then = self.expand_recursive(then_branch, depth + 1)?;
            let expanded_else = self.expand_recursive(else_branch, depth + 1)?;
            return Ok(Expr::If {
                condition: Box::new(expanded_condition),
                then_branch: Box::new(expanded_then),
                else_branch: Box::new(expanded_else),
                span: span.clone(),
            });
        }

        // Now, handle lists, which are the primary target for macro expansion.
        let (items, span) = match expr {
            Expr::List(items, span) => (items, span),
            // For any other expression type (symbols, literals), there's nothing to expand.
            _ => return Ok(expr.clone()),
        };

        if items.is_empty() {
            return Ok(expr.clone());
        }

        // Check if the head of the list is a registered macro.
        if let Some(Expr::Symbol(s, _)) = items.get(0) {
            if let Some(macro_def) = self.macros.get(s) {
                // It's a macro call. Expand it once.
                let expanded = match macro_def {
                    MacroDef::Fn(func) => func(expr)?,
                    MacroDef::Template(template) => {
                        self.expand_template(template, expr, depth)?
                    }
                };
                // The result of the expansion might itself be another macro call,
                // so we must recurse on the *new* expression.
                return self.expand_recursive(&expanded, depth + 1);
            }
        }

        // If it's not a macro call, it's a regular list (like an atom call or data).
        // We need to recursively expand its children.
        let expanded_items = items
            .iter()
            .map(|item| self.expand_recursive(item, depth + 1))
            .collect::<Result<Vec<Expr>, _>>()?;

        Ok(Expr::List(expanded_items, span.clone()))
    }

    /// Provides a step-by-step trace of the macro expansion process.
    ///
    /// This is a powerful debugging tool for authors to understand how their
    /// code is being transformed. It returns a vector of `TraceStep` structs,
    /// each representing a single expansion step.
    pub fn macroexpand_trace(&self, expr: &Expr) -> Result<Vec<TraceStep>, SutraError> {
        let mut trace = Vec::new();
        trace.push(TraceStep {
            description: "Initial expression".to_string(),
            ast: expr.clone(),
        });

        self.trace_recursive(expr, &mut trace, 0)?;

        // Add the final, fully expanded form as the last step for clarity.
        if let Some(last_step) = trace.last() {
            if last_step.description != "Final expanded form" {
                let final_ast = self.expand_recursive(expr, 0)?;
                trace.push(TraceStep {
                    description: "Final expanded form".to_string(),
                    ast: final_ast,
                });
            }
        }

        Ok(trace)
    }

    fn expand_template(
        &self,
        template: &MacroTemplate,
        expr: &Expr,
        depth: usize,
    ) -> Result<Expr, SutraError> {
        let (items, span) = match expr {
            Expr::List(items, span) => (items, span),
            _ => {
                return Err(SutraError {
                    kind: SutraErrorKind::Macro(
                        "Template macro must be called as a list.".to_string(),
                    ),
                    span: Some(expr.span()),
                });
            }
        };

        let args = &items[1..];

        // Arity check
        if args.len() < template.params.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects at least {} arguments, but got {}.",
                    template.params.len(),
                    args.len()
                )),
                span: Some(span.clone()),
            });
        }

        if template.variadic_param.is_none() && args.len() > template.params.len() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro expects exactly {} arguments, but got {}.",
                    template.params.len(),
                    args.len()
                )),
                span: Some(span.clone()),
            });
        }

        let mut bindings = HashMap::new();
        for (i, param_name) in template.params.iter().enumerate() {
            bindings.insert(param_name.clone(), args[i].clone());
        }

        if let Some(variadic_name) = &template.variadic_param {
            let rest_args = args[template.params.len()..].to_vec();
            bindings.insert(
                variadic_name.clone(),
                Expr::List(rest_args, span.clone()),
            );
        }

        let substituted_body = self.substitute(&template.body, &bindings)?;
        self.expand_recursive(&substituted_body, depth + 1)
    }

    fn substitute(
        &self,
        expr: &Expr,
        bindings: &HashMap<String, Expr>,
    ) -> Result<Expr, SutraError> {
        match expr {
            Expr::Symbol(name, span) => {
                if let Some(bound_expr) = bindings.get(name) {
                    Ok(bound_expr.clone())
                } else {
                    Ok(Expr::Symbol(name.clone(), span.clone()))
                }
            }
            Expr::List(items, span) => {
                let new_items = items
                    .iter()
                    .map(|item| self.substitute(item, bindings))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expr::List(new_items, span.clone()))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let new_condition = self.substitute(condition, bindings)?;
                let new_then = self.substitute(then_branch, bindings)?;
                let new_else = self.substitute(else_branch, bindings)?;
                Ok(Expr::If {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: Box::new(new_else),
                    span: span.clone(),
                })
            }
            // Literals and paths are returned as-is
            _ => Ok(expr.clone()),
        }
    }

    /// The recursive implementation for `macroexpand_trace`.
    fn trace_recursive(
        &self,
        expr: &Expr,
        trace: &mut Vec<TraceStep>,
        depth: usize,
    ) -> Result<Expr, SutraError> {
        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            return Err(SutraError {
                kind: SutraErrorKind::Macro("Macro expansion depth limit exceeded.".to_string()),
                span: Some(expr.span()),
            });
        }

        if let Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } = expr
        {
            let expanded_condition = self.trace_recursive(condition, trace, depth + 1)?;
            let expanded_then = self.trace_recursive(then_branch, trace, depth + 1)?;
            let expanded_else = self.trace_recursive(else_branch, trace, depth + 1)?;
            return Ok(Expr::If {
                condition: Box::new(expanded_condition),
                then_branch: Box::new(expanded_then),
                else_branch: Box::new(expanded_else),
                span: span.clone(),
            });
        }

        let (items, span) = match expr {
            Expr::List(items, span) => (items, span),
            _ => return Ok(expr.clone()),
        };

        if items.is_empty() {
            return Ok(expr.clone());
        }

        if let Some(Expr::Symbol(s, _)) = items.get(0) {
            if let Some(macro_def) = self.macros.get(s) {
                let expanded = match macro_def {
                    MacroDef::Fn(func) => func(expr)?,
                    MacroDef::Template(template) => self.expand_template(template, expr, depth)?,
                };
                trace.push(TraceStep {
                    description: format!("Expanding macro `{}`", s),
                    ast: expanded.clone(),
                });
                return self.trace_recursive(&expanded, trace, depth + 1);
            }
        }

        let expanded_items = items
            .iter()
            .map(|item| self.trace_recursive(item, trace, depth + 1))
            .collect::<Result<Vec<Expr>, _>>()?;

        Ok(Expr::List(expanded_items, span.clone()))
    }
}
