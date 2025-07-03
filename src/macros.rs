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
use std::fs;
use sha2::{Digest, Sha256};

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

        // Arity check: too few arguments
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

        // Too many arguments for non-variadic macro
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

        // Bind fixed parameters positionally
        let mut bindings = HashMap::new();
        for (i, param_name) in template.params.iter().enumerate() {
            bindings.insert(param_name.clone(), args[i].clone());
        }

        // Bind variadic parameter (if present) to a list of remaining args (may be empty)
        if let Some(variadic_name) = &template.variadic_param {
            let rest_args = if args.len() > template.params.len() {
                args[template.params.len()..].to_vec()
            } else {
                vec![]
            };
            bindings.insert(
                variadic_name.clone(),
                Expr::List(rest_args, span.clone()),
            );
        }

        // TODO: Consider moving arity/positional validation to MacroTemplate::new if possible.

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

    /// Computes a SHA256 hash of all macro names and their source/expansion forms, sorted deterministically.
    pub fn hash(&self) -> String {
        let mut entries: Vec<(String, String)> = self.macros.iter().map(|(name, def)| {
            let def_str = match def {
                MacroDef::Template(template) => {
                    // Serialize params, variadic_param, and body in a stable way
                    let mut s = String::new();
                    s.push_str(&format!("params:{:?};", template.params));
                    s.push_str(&format!("variadic:{:?};", template.variadic_param));
                    s.push_str(&format!("body:{};", template.body.pretty()));
                    s
                }
                MacroDef::Fn(_) => "native_fn".to_string(),
            };
            (name.clone(), def_str)
        }).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut hasher = Sha256::new();
        for (name, def_str) in entries {
            hasher.update(name.as_bytes());
            hasher.update(def_str.as_bytes());
        }
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

impl MacroTemplate {
    /// Constructs a MacroTemplate with validation for variadic and duplicate parameters.
    pub fn new(
        params: Vec<String>,
        variadic_param: Option<String>,
        body: Box<Expr>,
    ) -> Result<Self, SutraError> {
        // Check for duplicate parameter names
        let mut all_names = params.clone();
        if let Some(var) = &variadic_param {
            all_names.push(var.clone());
        }
        let mut seen = std::collections::HashSet::new();
        for name in &all_names {
            if !seen.insert(name) {
                return Err(SutraError {
                    kind: SutraErrorKind::Macro(format!(
                        "Duplicate parameter name '{}' in macro definition.",
                        name
                    )),
                    span: None,
                });
            }
        }
        // If variadic, it must be last
        if variadic_param.is_some() && !params.is_empty() {
            // (a b . rest) is valid, but (a . rest b) is not
            // This is enforced by the parser, but double-check here
            // (We assume the parser provides params as all fixed, variadic_param as Option)
        }
        Ok(MacroTemplate {
            params,
            variadic_param,
            body,
        })
    }
}

/// Pure loader: parses macro definitions from a Sutra source string.
pub fn parse_macros_from_source(source: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    use crate::parser;
    use crate::ast::Expr;
    use std::collections::{HashMap, HashSet};

    let exprs = parser::parse(source)?;
    let mut macros = Vec::new();
    let mut names_seen = HashSet::new();

    for expr in exprs {
        // Only process (define (name ...) body) forms
        if let Expr::List(items, span) = &expr {
            if items.len() == 3 {
                if let Expr::Symbol(def, _) = &items[0] {
                    if def == "define" {
                        // Parse parameter list
                        if let Expr::List(param_items, _) = &items[1] {
                            let mut iter = param_items.iter();
                            let macro_name = match iter.next() {
                                Some(Expr::Symbol(name, _)) => name.clone(),
                                _ => {
                                    return Err(SutraError {
                                        kind: SutraErrorKind::Macro("Macro name must be a symbol as the first element of the parameter list.".to_string()),
                                        span: Some(items[1].span()),
                                    });
                                }
                            };
                            if !names_seen.insert(macro_name.clone()) {
                                return Err(SutraError {
                                    kind: SutraErrorKind::Macro(format!("Duplicate macro name '{}'.", macro_name)),
                                    span: Some(items[1].span()),
                                });
                            }
                            // Use MacroParams::parse_macro_params for all parameter parsing/validation
                            let macro_head = Some(items[1].clone());
                            // DEBUG: Print macro name and param items
                            eprintln!("[DEBUG] macro_name: {:?}", macro_name);
                            eprintln!("[DEBUG] param_items: {:?}", param_items);
                            let macro_params = MacroParams::parse_macro_params(
                                &param_items[1..],
                                macro_head.clone(),
                                Some(items[1].span()),
                            )?;
                            // DEBUG: Print parsed params and variadic
                            eprintln!("[DEBUG] macro_params.params: {:?}", macro_params.params);
                            eprintln!("[DEBUG] macro_params.variadic: {:?}", macro_params.variadic);
                            let template = MacroTemplate::new(
                                macro_params.params,
                                macro_params.variadic,
                                Box::new(items[2].clone()),
                            )?;
                            macros.push((macro_name, template));
                        } else {
                            return Err(SutraError {
                                kind: SutraErrorKind::Macro("Macro parameter list must be a list.".to_string()),
                                span: Some(items[1].span()),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(macros)
}

/// Thin wrapper: loads macro definitions from a file.
pub fn load_macros_from_file(path: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path)
        .map_err(|e| SutraError { kind: SutraErrorKind::Io(e.to_string()), span: None })?;
    parse_macros_from_source(&source)
}

#[derive(Debug, Clone)]
pub struct MacroParams {
    pub params: Vec<String>,
    pub variadic: Option<String>,
    pub head: Option<Expr>,
    pub span: Option<crate::ast::Span>,
}

pub const RESERVED_WORDS: &[&str] = &[".", "define"];

impl MacroParams {
    /// Parse and validate macro parameters from a list of Exprs (excluding macro name).
    /// Returns MacroParams or a SutraError with full context.
    pub fn parse_macro_params(param_exprs: &[Expr], macro_head: Option<Expr>, span: Option<crate::ast::Span>) -> Result<Self, SutraError> {
        let mut params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut variadic = None;
        let mut found_dot = false;
        let mut iter = param_exprs.iter().peekable();
        while let Some(item) = iter.next() {
            match item {
                Expr::Symbol(s, sspan) if s == "." => {
                    if found_dot {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(format!("Multiple '.' in parameter list: {}", MacroParams::format_head(&macro_head))),
                            span: span.clone().or_else(|| Some(sspan.clone())),
                        });
                    }
                    found_dot = true;
                    match iter.next() {
                        Some(Expr::Symbol(var, vspan)) => {
                            if RESERVED_WORDS.contains(&var.as_str()) {
                                return Err(SutraError {
                                    kind: SutraErrorKind::Macro(format!("Reserved word '{}' used as variadic parameter: {}", var, MacroParams::format_head(&macro_head))),
                                    span: span.clone().or_else(|| Some(vspan.clone())),
                                });
                            }
                            if !seen.insert(var.clone()) {
                                return Err(SutraError {
                                    kind: SutraErrorKind::Macro(format!("Duplicate parameter name '{}' in macro definition: {}", var, MacroParams::format_head(&macro_head))),
                                    span: span.clone().or_else(|| Some(vspan.clone())),
                                });
                            }
                            variadic = Some(var.clone());
                            // After variadic, check for any further items
                            if iter.peek().is_some() {
                                // There are parameters after the variadic
                                let rem = iter.next().unwrap();
                                return Err(SutraError {
                                    kind: SutraErrorKind::Macro(format!("No parameters allowed after variadic parameter in macro definition: {}", MacroParams::format_head(&macro_head))),
                                    span: span.clone().or_else(|| Some(rem.span())),
                                });
                            }
                            break;
                        }
                        Some(bad) => {
                            return Err(SutraError {
                                kind: SutraErrorKind::Macro(format!("Expected symbol after '.' in parameter list: {}", MacroParams::format_head(&macro_head))),
                                span: span.clone().or_else(|| Some(bad.span())),
                            });
                        }
                        None => {
                            return Err(SutraError {
                                kind: SutraErrorKind::Macro(format!("Expected symbol after '.' in parameter list, got end of list: {}", MacroParams::format_head(&macro_head))),
                                span: span.clone(),
                            });
                        }
                    }
                }
                Expr::Symbol(s, sspan) => {
                    if found_dot {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(format!("No parameters allowed after variadic parameter in macro definition: {}", MacroParams::format_head(&macro_head))),
                            span: span.clone().or_else(|| Some(sspan.clone())),
                        });
                    }
                    if RESERVED_WORDS.contains(&s.as_str()) {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(format!("Reserved word '{}' used as parameter: {}", s, MacroParams::format_head(&macro_head))),
                            span: span.clone().or_else(|| Some(sspan.clone())),
                        });
                    }
                    if !seen.insert(s.clone()) {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(format!("Duplicate parameter name '{}' in macro definition: {}", s, MacroParams::format_head(&macro_head))),
                            span: span.clone().or_else(|| Some(sspan.clone())),
                        });
                    }
                    params.push(s.clone());
                }
                other => {
                    return Err(SutraError {
                        kind: SutraErrorKind::Macro(format!("Invalid parameter (must be symbol or '.'): {}", MacroParams::format_head(&macro_head))),
                        span: span.clone().or_else(|| Some(other.span())),
                    });
                }
            }
        }
        if found_dot && variadic.is_none() {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!("Expected symbol after '.' in parameter list: {}", MacroParams::format_head(&macro_head))),
                span,
            });
        }
        Ok(MacroParams { params, variadic, head: macro_head, span })
    }

    /// Helper to format macro head for error messages.
    pub fn format_head(head: &Option<Expr>) -> String {
        match head {
            Some(expr) => format!("{}", expr.pretty()),
            None => "<unknown macro head>".to_string(),
        }
    }
}
