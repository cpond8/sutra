//!
//! Centralized error message constants for the entire codebase.
//!
//! This module provides a single source of truth for all error messages,
//! organized by functional area.
//!
//! # Usage
//!
//! ```rust
//! use crate::error_messages::*;
//!
//! // Use constants directly
//! return Err(err_msg!(Validation, DUPLICATE_PARAMETER_NAME));
//!
//! // Use with string replacement
//! return Err(err_msg!(
//!     Validation,
//!     format!("{}", DUPLICATE_MACRO_NAME.replace("{}", &name))
//! ));
//! ```

// ============================================================================
// PARSING ERRORS
// ============================================================================

pub const ERROR_EMPTY_EXPR_PAIR: &str = "Empty expr pair: {}";
pub const ERROR_MISSING_PARAM_LIST: &str = "Missing parameter list";
pub const ERROR_MISSING_BODY: &str = "Missing body";
pub const ERROR_DEFINE_MUST_HAVE_PARAMLIST: &str = "Define form must have a ParamList";
pub const ERROR_DEFINE_MUST_HAVE_NAME: &str = "Define form must have a name";
pub const ERROR_MALFORMED_SPREAD: &str = "Malformed spread: missing symbol";
pub const ERROR_MALFORMED_QUOTE: &str = "Malformed quote: missing inner expression";
pub const ERROR_SPREAD_MISSING_SYMBOL: &str = "spread_arg: missing symbol after '...'";
pub const ERROR_REQUIRED_PARAM_AFTER_REST: &str = "Required parameter after rest";
pub const ERROR_INVALID_PARAM_ITEM: &str = "Invalid param item: {:?}";
pub const ERROR_NO_AST_BUILDER: &str = "No AST builder for rule: {:?}";
pub const ERROR_EMPTY_ATOM_PAIR: &str = "Empty atom pair";
pub const ERROR_NUMBER_PARSE: &str = "Number parse error: {}";
pub const ERROR_INVALID_BOOLEAN: &str = "Invalid boolean: {}";
pub const ERROR_PARSER_EMPTY_TREE: &str = "Parser generated an empty tree, this should not happen.";
pub const ERROR_INVALID_PATH: &str = "Invalid path";

// ============================================================================
// MACRO ERRORS
// ============================================================================

pub const ERROR_DUPLICATE_MACRO_NAME: &str = "Duplicate macro name '{}'";
pub const ERROR_DUPLICATE_PARAMETER_NAME: &str = "Duplicate parameter name";
pub const ERROR_MACRO_CALLABLE_INTERFACE: &str = "Macros cannot be called through Callable interface - they require AST transformation, not evaluation";
pub const ERROR_NOT_MACRO_DEFINITION_LIST: &str = "Not a macro definition list.";
pub const ERROR_MACRO_DEFINITION_WRONG_ELEMENTS: &str = "Macro definition must have 3 elements.";
pub const ERROR_MACRO_DEFINITION_FIRST_ELEMENT: &str = "First element must be 'define'.";
pub const ERROR_MACRO_NAME_MISSING: &str = "Macro name missing in parameter list.";
pub const ERROR_EXPECTED_LIST_FORM: &str = "Expected a list form for this macro";
pub const ERROR_DUPLICATE_MACRO_NAME_STANDARD_LIBRARY: &str =
    "Duplicate macro name '{}' in standard macro library.";
pub const ERROR_MACRO_NAME_CANNOT_BE_EMPTY: &str = "Macro name cannot be empty";

// ============================================================================
// EVALUATION ERRORS
// ============================================================================

pub const ERROR_ARITY_ERROR: &str = "Arity error";
pub const ERROR_TYPE_ERROR: &str = "Type error";
pub const ERROR_DIVISION_BY_ZERO: &str = "division by zero";
pub const ERROR_MODULO_BY_ZERO: &str = "modulo by zero";
pub const ERROR_CAR_EMPTY_LIST: &str = "car: empty list";
pub const ERROR_CDR_EMPTY_LIST: &str = "cdr: empty list";
pub const ERROR_LET_BINDING_SYMBOL: &str = "let: binding name must be a symbol";
pub const ERROR_SPECIAL_FORM_CALLABLE: &str =
    "Special Form atoms cannot be called through Callable interface - use direct dispatch instead";
pub const ERROR_EXPECTS_STRING_ARGUMENT: &str = "error expects a String argument";

// ============================================================================
// TEST ERRORS
// ============================================================================

pub const ERROR_DUPLICATE_TEST_NAME: &str = "Duplicate test name '{}'.";
pub const ERROR_MISSING_EXPECT_FORM: &str = "Test '{}' missing (expect ...) form";
pub const ERROR_TEST_REGISTRY_POISONED: &str = "Test registry mutex poisoned";
pub const ERROR_SOME_TESTS_FAILED: &str = "Some tests failed";

// ============================================================================
// CLI ERRORS
// ============================================================================

pub const ERROR_INVALID_FILENAME: &str = "Invalid filename";
pub const ERROR_FILE_READ: &str = "Failed to read file: {}";
pub const ERROR_GRAMMAR_VALIDATION: &str = "Failed to validate grammar: {}";

// ============================================================================
// VALIDATION ERRORS
// ============================================================================

pub const ERROR_VALIDATION_FAILED: &str = "Validation failed";
pub const ERROR_INVALID_MACRO_EXPANSION: &str = "Invalid macro expansion";

// ============================================================================
// INTERNAL ERRORS
// ============================================================================

pub const ERROR_INTERNAL_FAILURE: &str = "Internal error";
pub const ERROR_UNEXPECTED_STATE: &str = "Unexpected internal state";
