//! Internal builder functions for creating errors.
//!
//! These functions are called by the public API to construct the internal
//! error types with proper validation and defaults.

use super::internal::InternalSutraError;
use crate::errors::OldSutraError;
use miette::{NamedSource, SourceSpan};
use std::sync::Arc;

/// Build a ParseMissing error
pub(super) fn build_parse_missing(
    element: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::ParseMissing {
        element,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ParseMalformed error
pub(super) fn build_parse_malformed(
    construct: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::ParseMalformed {
        construct,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ParseInvalidValue error
pub(super) fn build_parse_invalid_value(
    item_type: String,
    value: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::ParseInvalidValue {
        item_type,
        value,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a RuntimeUndefinedSymbol error
pub(super) fn build_runtime_undefined_symbol(
    symbol: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::RuntimeUndefinedSymbol {
        symbol,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a RuntimeGeneral error
pub(super) fn build_runtime_general(
    message: String,
    label: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::RuntimeGeneral {
        message,
        label,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ValidationArity error
pub(super) fn build_validation_arity(
    expected: String,
    actual: usize,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::ValidationArity {
        expected,
        actual,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ParseEmpty error
pub(super) fn build_parse_empty(src: Arc<NamedSource<String>>, span: SourceSpan) -> OldSutraError {
    OldSutraError(InternalSutraError::ParseEmpty {
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ParseParameterOrder error
pub(super) fn build_parse_parameter_order(
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
    rest_span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::ParseParameterOrder {
        src,
        span,
        rest_span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a TypeMismatch error
pub(super) fn build_type_mismatch(
    expected: String,
    actual: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    OldSutraError(InternalSutraError::TypeMismatch {
        expected,
        actual,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a TestAssertion error
pub(super) fn build_test_assertion(
    message: String,
    test_name: String,
    src: Arc<NamedSource<String>>,
    span: SourceSpan,
) -> OldSutraError {
    let test_file = src.name().to_string();
    OldSutraError(InternalSutraError::TestAssertion {
        message,
        test_name,
        test_file,
        src,
        span,
        help: None,
        related_spans: Vec::new(),
        is_warning: false,
    })
}
