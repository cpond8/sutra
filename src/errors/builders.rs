//! Internal builder functions for creating errors.
//!
//! These functions are called by the public API to construct the internal
//! error types with proper validation and defaults.

use super::internal::InternalSutraError;
use crate::errors::SutraError;
use miette::{NamedSource, SourceSpan};
use std::sync::Arc;

/// Build a ParseMissing error
pub(super) fn build_parse_missing(
    element: String,
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_parse_missing");
    }

    SutraError(InternalSutraError::ParseMissing {
        element,
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_parse_malformed");
    }

    SutraError(InternalSutraError::ParseMalformed {
        construct,
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_parse_invalid_value");
    }

    SutraError(InternalSutraError::ParseInvalidValue {
        item_type,
        value,
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_runtime_undefined_symbol");
    }

    SutraError(InternalSutraError::RuntimeUndefinedSymbol {
        symbol,
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_runtime_general");
    }

    SutraError(InternalSutraError::RuntimeGeneral {
        message,
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_validation_arity");
    }

    SutraError(InternalSutraError::ValidationArity {
        expected,
        actual,
        src: Arc::new(NamedSource::new(source_name, source_code)),
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}

/// Build a ParseEmpty error
pub(super) fn build_parse_empty(
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_parse_empty");
    }

    SutraError(InternalSutraError::ParseEmpty {
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
    rest_span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_parse_parameter_order");
    }

    SutraError(InternalSutraError::ParseParameterOrder {
        src: Arc::new(NamedSource::new(source_name, source_code)),
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
    source_name: String,
    source_code: String,
    span: SourceSpan,
) -> SutraError {
    if source_code.is_empty() {
        panic!("Internal error: empty source code provided to build_type_mismatch. Expected: '{}', Actual: '{}'", expected, actual);
    }

    SutraError(InternalSutraError::TypeMismatch {
        expected,
        actual,
        src: Arc::new(NamedSource::new(source_name, source_code)),
        span,
        help: None,
        related_spans: Vec::new(),
        test_file: None,
        test_name: None,
        is_warning: false,
    })
}