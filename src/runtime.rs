//! Runtime module for the Sutra language
//!
//! This module provides the core runtime value types for the Sutra engine.
//! All computation, macro expansion, and evaluation produces or manipulates these types.
//! Values are deeply compositional: lists and maps can contain any other value.

use std::{collections::HashMap, fmt, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::errors::SutraError;
use crate::{AstNode, ParamList, Path, Span};

/// Unified native function signature.
///
/// All native functions (atoms) adhere to this signature. They receive unevaluated
/// `AstNode` arguments and the evaluation context. They are responsible for evaluating
/// their arguments as needed, allowing for both lazy and eager evaluation strategies
/// within a single, consistent framework.
pub type NativeFn =
    fn(args: &[AstNode], context: &mut EvaluationContext, call_span: &Span) -> SpannedResult;

/// Canonical runtime value for Sutra evaluation and macro expansion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Value {
    /// Absence of a value; default for uninitialized slots.
    #[default]
    Nil,
    /// Numeric value (floating point).
    Number(f64),
    /// String value.
    String(String),
    /// Boolean value.
    Bool(bool),
    /// A Lisp-style cons cell, forming the head of a list.
    Cons(ConsRepr),
    /// Map from string keys to values (deeply compositional).
    Map(HashMap<String, Value>),
    /// Reference to a path in the world state (not auto-resolved).
    Path(Path),
    /// User-defined lambda function (captures parameter list and body).
    Lambda(Rc<Lambda>),
    /// Native (Rust) function.
    #[serde(skip)]
    NativeFn(NativeFn),
    Symbol(String),
    Quote(Box<Value>),
}

/// Represents a user-defined lambda function (for closures and function values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lambda {
    pub params: ParamList,  // Parameter names, variadic info
    pub body: Box<AstNode>, // The function body (AST)
    pub captured_env: HashMap<String, Value>,
}

/// Represents a Lisp-style cons cell (a pair of values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsCell {
    pub car: Value,
    pub cdr: Value,
}

/// Internal optimized representation for cons cells
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsRepr {
    /// Single element list (most common case)
    Single(Box<Value>),
    /// General case for longer lists
    Chain(Rc<ConsCell>),
}

impl ConsRepr {
    /// Get the car (first element) of the cons cell
    pub fn car(&self) -> &Value {
        match self {
            ConsRepr::Single(val) => val,
            ConsRepr::Chain(cell) => &cell.car,
        }
    }

    /// Get the cdr (rest) of the cons cell
    pub fn cdr(&self) -> Value {
        match self {
            ConsRepr::Single(_) => Value::Nil,
            ConsRepr::Chain(cell) => cell.cdr.clone(),
        }
    }

    /// Create a new cons cell with optimized representation
    pub fn cons(car: Value, cdr: Value) -> ConsRepr {
        match cdr {
            Value::Nil => ConsRepr::Single(Box::new(car)),
            Value::Cons(ConsRepr::Single(existing)) => {
                // Create a proper two-element list where cdr points to single-element list
                ConsRepr::Chain(Rc::new(ConsCell {
                    car,
                    cdr: Value::Cons(ConsRepr::Single(existing)),
                }))
            }
            _ => ConsRepr::Chain(Rc::new(ConsCell { car, cdr })),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Cons(a), Value::Cons(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Path(a), Value::Path(b)) => a == b,
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (Value::NativeFn(a), Value::NativeFn(b)) => *a as usize == *b as usize,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Quote(a), Value::Quote(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    /// Returns the type name of the value as a string (for diagnostics, debugging, and macro logic).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Cons(_) => "List", // User-facing type name remains "List" for consistency.
            Value::Map(_) => "Map",
            Value::Path(_) => "Path",
            Value::Lambda(_) => "Lambda",
            Value::NativeFn(_) => "NativeFn",
            Value::Symbol(_) => "Symbol",
            Value::Quote(_) => "Quote",
        }
    }

    /// Returns true if the value is Nil (used for default checks and absence semantics).
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Returns true if the value is considered "truthy" in a boolean context.
    /// In Sutra, `nil`, `false`, `0`, empty strings, and empty collections are falsy.
    /// Everything else is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false, // This includes empty lists
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Quote(inner) => inner.is_truthy(),
            _ => true,
        }
    }

    /// Returns the contained number if this is a Number value, else None.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns a reference to the contained string if this is a String value, else None.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns a reference to the contained map if this is a Map value, else None.
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the list as a slice if this is a List value, else None.
    /// This provides a simpler interface than cons cell iteration.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            _ => None,
        }
    }

    /// Generic type checking with clear error messages for common patterns.
    pub fn expect_type(&self, expected: &str) -> Result<&Self, String> {
        let actual = self.type_name();
        if self.matches_type_name(expected) {
            Ok(self)
        } else {
            Err(format!("expected {expected}, got {actual}"))
        }
    }

    /// Helper to check if value matches a type name (handles List/Cons duality).
    fn matches_type_name(&self, expected: &str) -> bool {
        let actual = self.type_name();
        actual == expected || (expected == "List" && matches!(self, Value::Cons(_)))
    }

    /// Creates a list from a vector of values.
    pub fn from_list(items: Vec<Value>) -> Self {
        let mut result = Value::Nil;
        for item in items.into_iter().rev() {
            result = Value::Cons(ConsRepr::cons(item, result));
        }
        result
    }

    /// Attempts to create an iterator over a `Value`.
    ///
    /// If the `Value` is a `Cons`, `List`, or `Nil`, it returns an iterator.
    /// This is the primary way to traverse list structures.
    pub fn try_into_iter(self) -> impl Iterator<Item = Value> {
        ValueIterator::new(self)
    }

    // ------------------------------------------------------------------------
    // Display formatting helpers (internal)
    // ------------------------------------------------------------------------

    fn fmt_list_like(f: &mut fmt::Formatter<'_>, value: &Value) -> fmt::Result {
        write!(f, "(")?;
        let items: Vec<_> = value.clone().try_into_iter().collect();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{item}")?;
        }
        write!(f, ")")
    }

    fn fmt_map(f: &mut fmt::Formatter<'_>, map: &HashMap<String, Value>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in map.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Cons(_) => Value::fmt_list_like(f, self),
            Value::Map(map) => Value::fmt_map(f, map),
            Value::Path(p) => write!(f, "{p}"),
            Value::Lambda(_) => write!(f, "<lambda>"),
            Value::NativeFn(_) => write!(f, "<native_fn>"),
            Value::Symbol(s) => write!(f, "{s}"),
            Value::Quote(v) => write!(f, "'{v}"),
        }
    }
}

// ============================================================================
// SPANNED VALUE TYPES
// ============================================================================

/// A canonical value paired with its source span. By carrying the span with the
/// value, we ensure that any subsequent errors related to this value (e.g.,
/// type mismatches) can be reported with precise source location information.
#[derive(Debug, Clone)]
pub struct SpannedValue {
    pub value: Value,
    pub span: Span,
}

impl SpannedValue {
    /// Create a new SpannedValue with the given value and span.
    pub fn new(value: Value, span: Span) -> Self {
        Self { value, span }
    }

    /// Create a SpannedValue containing Nil.
    pub fn nil(span: Span) -> Self {
        Self {
            value: Value::Nil,
            span,
        }
    }

    /// Create a SpannedValue containing a number.
    pub fn number(n: f64, span: Span) -> Self {
        Self {
            value: Value::Number(n),
            span,
        }
    }

    /// Create a SpannedValue containing a boolean.
    pub fn bool(b: bool, span: Span) -> Self {
        Self {
            value: Value::Bool(b),
            span,
        }
    }

    /// Create a SpannedValue containing a string.
    pub fn string(s: String, span: Span) -> Self {
        Self {
            value: Value::String(s),
            span,
        }
    }

    /// Unwrap as a number with context-aware error reporting.
    pub fn unwrap_number(self, ctx: &ErrorContext) -> Result<f64, crate::errors::SutraError> {
        match self.value {
            Value::Number(n) => Ok(n),
            _ => Err(ctx.type_mismatch(
                "Number",
                self.value.type_name(),
                crate::errors::to_source_span(self.span),
            )),
        }
    }

    /// Unwrap as a string with context-aware error reporting.
    pub fn unwrap_string(self, ctx: &ErrorContext) -> Result<String, crate::errors::SutraError> {
        match self.value {
            Value::String(s) => Ok(s),
            _ => Err(ctx.type_mismatch(
                "String",
                self.value.type_name(),
                crate::errors::to_source_span(self.span),
            )),
        }
    }

    /// Unwrap as a boolean with context-aware error reporting.
    pub fn unwrap_bool(self, ctx: &ErrorContext) -> Result<bool, crate::errors::SutraError> {
        match self.value {
            Value::Bool(b) => Ok(b),
            _ => Err(ctx.type_mismatch(
                "Bool",
                self.value.type_name(),
                crate::errors::to_source_span(self.span),
            )),
        }
    }

    /// Check if the value is truthy (useful for conditions).
    pub fn is_truthy(&self) -> bool {
        self.value.is_truthy()
    }
}

/// The canonical result type for any operation that produces a value.
pub type SpannedResult = Result<SpannedValue, SutraError>;

/// Internal iterator implementation for Value lists.
/// This is a more efficient implementation that handles different value types optimally.
enum ValueIterator {
    Cons { current: Value },
    Empty,
}

impl ValueIterator {
    fn new(value: Value) -> Self {
        match value {
            Value::Cons(_) => ValueIterator::Cons { current: value },
            Value::Nil => ValueIterator::Empty,
            _ => ValueIterator::Empty,
        }
    }
}

impl Iterator for ValueIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ValueIterator::Cons { current } => {
                match current {
                    Value::Cons(cell) => {
                        let car = cell.car().clone();
                        // Move to the next link in the chain
                        *current = cell.cdr();
                        Some(car)
                    }
                    Value::Nil => None,
                    _ => {
                        // Non-list value, end iteration
                        None
                    }
                }
            }
            ValueIterator::Empty => None,
        }
    }
}

// ============================================================================
// EVALUATION CONTEXT - Simplified evaluation state
// ============================================================================

/// Simplified evaluation context with essential state only
pub struct EvaluationContext {
    pub world: crate::prelude::CanonicalWorld,
    pub output: crate::atoms::SharedOutput,
    pub source: crate::errors::SourceContext,
    pub depth: usize,
    pub max_depth: usize,
    pub env: Vec<std::collections::HashMap<String, Value>>, // Stack of lexical scopes
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(
        world: crate::prelude::CanonicalWorld,
        output: crate::atoms::SharedOutput,
        source: crate::errors::SourceContext,
    ) -> Self {
        let mut global_env = std::collections::HashMap::new();
        global_env.insert("nil".to_string(), Value::Nil);

        Self {
            world,
            output,
            source,
            depth: 0,
            max_depth: 1000,
            env: vec![global_env],
        }
    }

    /// Create context with custom settings
    pub fn with_settings(
        world: crate::prelude::CanonicalWorld,
        output: crate::atoms::SharedOutput,
        source: crate::errors::SourceContext,
        max_depth: usize,
    ) -> Self {
        let mut ctx = Self::new(world, output, source);
        ctx.max_depth = max_depth;
        ctx
    }

    /// Set a variable in the current lexical scope
    pub fn set_var(&mut self, name: &str, value: Value) {
        if let Some(current_scope) = self.env.last_mut() {
            current_scope.insert(name.to_string(), value);
        }
    }

    /// Get a variable from the lexical scope chain (search from innermost to outermost)
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        // Search from innermost scope to outermost
        for scope in self.env.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        None
    }

    /// Create a new lexical frame for let/lambda
    pub fn with_new_frame(&self) -> Self {
        let mut new_env = self.env.clone();
        new_env.push(std::collections::HashMap::new()); // Add new lexical scope

        Self {
            world: std::rc::Rc::clone(&self.world),
            output: self.output.clone(),
            source: self.source.clone(),
            depth: self.depth,
            max_depth: self.max_depth,
            env: new_env,
        }
    }

    /// Extract span information for error reporting
    pub fn span_for_node(&self, node: &AstNode) -> miette::SourceSpan {
        crate::errors::to_source_span(node.span)
    }
}

impl crate::errors::ErrorReporting for EvaluationContext {
    fn report(
        &self,
        kind: crate::errors::ErrorKind,
        span: miette::SourceSpan,
    ) -> crate::errors::SutraError {
        use crate::errors::{DiagnosticInfo, SourceInfo, SutraError};

        SutraError {
            kind: kind.clone(),
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                phase: "runtime".into(),
            },
            diagnostic_info: DiagnosticInfo {
                help: None,
                error_code: format!("sutra::runtime::{}", kind.code_suffix()),
            },
        }
    }
}

/// Separate error creation context for cleaner error handling patterns.
/// This provides a cleaner separation of concerns than embedding error reporting
/// directly in the evaluation context.
pub struct ErrorContext<'a> {
    pub source: &'a crate::errors::SourceContext,
    pub phase: &'static str,
}

impl<'a> ErrorContext<'a> {
    /// Create an error context from an evaluation context.
    pub fn from_eval_context(ctx: &'a EvaluationContext) -> Self {
        Self {
            source: &ctx.source,
            phase: "runtime",
        }
    }

    /// Create a type mismatch error with clean interface.
    pub fn type_mismatch(
        &self,
        expected: &str,
        actual: &str,
        span: miette::SourceSpan,
    ) -> crate::errors::SutraError {
        use crate::errors::{DiagnosticInfo, ErrorKind, SourceInfo, SutraError};

        SutraError {
            kind: ErrorKind::TypeMismatch {
                expected: expected.to_string(),
                actual: actual.to_string(),
            },
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                phase: self.phase.into(),
            },
            diagnostic_info: DiagnosticInfo {
                help: None,
                error_code: format!("sutra::{}::type_mismatch", self.phase),
            },
        }
    }

    /// Create an arity mismatch error.
    pub fn arity_mismatch(
        &self,
        expected: &str,
        actual: usize,
        span: miette::SourceSpan,
    ) -> crate::errors::SutraError {
        use crate::errors::{DiagnosticInfo, ErrorKind, SourceInfo, SutraError};

        SutraError {
            kind: ErrorKind::ArityMismatch {
                expected: expected.to_string(),
                actual,
            },
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                phase: self.phase.into(),
            },
            diagnostic_info: DiagnosticInfo {
                help: None,
                error_code: format!("sutra::{}::arity_mismatch", self.phase),
            },
        }
    }
}

// ============================================================================
// CORE EVALUATION FUNCTIONS
// ============================================================================

/// Core recursive evaluator
pub fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> SpannedResult {
    use crate::{errors::ErrorReporting, Expr};

    // Check recursion limit
    if context.depth > context.max_depth {
        return Err(context.report(
            crate::errors::ErrorKind::RecursionLimit,
            context.span_for_node(expr),
        ));
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
            crate::errors::ErrorKind::InvalidOperation {
                operation: "evaluate".to_string(),
                operand_type: expr.value.type_name().to_string(),
            },
            context.span_for_node(expr),
        )),
    }
}

/// Evaluate function calls
fn evaluate_call(items: &[AstNode], context: &mut EvaluationContext) -> SpannedResult {
    use crate::{errors::ErrorReporting, Expr};

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
            crate::atoms::special_forms::call_lambda(&lambda, &args, context, &head.span)
        }
        Value::NativeFn(func) => {
            // Pass unevaluated arguments to native function
            func(tail, context, &head.span)
        }
        _ => Err(context.report(
            crate::errors::ErrorKind::TypeMismatch {
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
    use crate::errors::ErrorReporting;

    // Check local environment first
    if let Some(value) = context.get_var(name) {
        return Ok(SpannedValue {
            value: value.clone(),
            span: node.span,
        });
    }

    // Check global world state
    let world_path = crate::atoms::Path(vec![name.to_string()]);
    if let Some(value) = context.world.borrow().state.get(&world_path) {
        return Ok(SpannedValue {
            value: value.clone(),
            span: node.span,
        });
    }

    // Undefined
    Err(context.report(
        crate::errors::ErrorKind::UndefinedSymbol {
            symbol: name.to_string(),
        },
        context.span_for_node(node),
    ))
}

/// Convert AST to quoted value
fn ast_to_value(node: &AstNode) -> Value {
    use crate::Expr;

    match &*node.value {
        Expr::Symbol(s, _) => Value::Symbol(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::List(items, _) => {
            let mut result = Value::Nil;
            for item in items.iter().rev() {
                let car_value = ast_to_value(item);
                result = Value::Cons(ConsRepr::cons(car_value, result));
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
    Ok(result.value.is_truthy())
}

// ============================================================================
// PUBLIC EVALUATION API
// ============================================================================

/// Main evaluation entry point
///
/// This is the primary interface for evaluating AST nodes. It creates an evaluation
/// context and delegates to the core evaluation logic.
pub fn evaluate(
    expr: &AstNode,
    world: crate::prelude::CanonicalWorld,
    output: crate::atoms::SharedOutput,
    source: crate::errors::SourceContext,
) -> Result<Value, crate::errors::SutraError> {
    let mut context = EvaluationContext::new(world, output, source);
    let result = evaluate_ast_node(expr, &mut context)?;
    Ok(result.value)
}
