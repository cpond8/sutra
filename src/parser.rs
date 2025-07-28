//! Sutra Parser - Simplified Implementation
//!
//! Converts Sutra source code into Abstract Syntax Tree nodes with source location tracking.
//! This parser is purely syntactic - no semantic analysis or type checking.

use crate::errors::{to_source_span, ErrorKind, SourceContext, SutraError};
use crate::{prelude::*, syntax::ParamList};
use pest::{error::Error, iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/grammar.pest"]
struct SutraParser;

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parse Sutra source code into AST nodes
pub fn parse(source_text: &str, source_context: SourceContext) -> Result<Vec<AstNode>, SutraError> {
    let pairs = SutraParser::parse(Rule::program, source_text)
        .map_err(|e| parse_error(e, &source_context))?;

    let program = pairs.peek().unwrap(); // pest guarantees program rule exists

    program
        .into_inner()
        .filter(|p| p.as_rule() != Rule::EOI)
        .map(|p| build_node(p, &source_context))
        .collect()
}

/// Wrap multiple AST nodes in a (do ...) form if needed
pub fn wrap_in_do(nodes: Vec<AstNode>) -> AstNode {
    match nodes.len() {
        0 => {
            let span = Span { start: 0, end: 0 };
            Spanned {
                value: Expr::List(vec![], span).into(),
                span,
            }
        }
        1 => nodes.into_iter().next().unwrap(),
        _ => {
            let span = calculate_span(&nodes);
            let do_symbol = Spanned {
                value: Expr::Symbol("do".to_string(), span).into(),
                span,
            };
            let mut items = Vec::with_capacity(nodes.len() + 1);
            items.push(do_symbol);
            items.extend(nodes);
            Spanned {
                value: Expr::List(items, span).into(),
                span,
            }
        }
    }
}

fn calculate_span(nodes: &[AstNode]) -> Span {
    if nodes.is_empty() {
        return Span { start: 0, end: 0 };
    }
    Span {
        start: nodes.first().unwrap().span.start,
        end: nodes.last().unwrap().span.end,
    }
}

// ============================================================================
// AST BUILDING
// ============================================================================

fn build_node(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    let span = extract_span(&pair);

    let expr = match pair.as_rule() {
        // Skip wrapper rules - grammar should handle this directly
        Rule::expr | Rule::atom => {
            let inner = pair.into_inner().next().unwrap();
            return build_node(inner, source);
        }

        Rule::number => {
            let value = pair
                .as_str()
                .parse::<f64>()
                .map_err(|_| invalid_literal_error(source, "number", pair.as_str(), span))?;
            Expr::Number(value, span)
        }

        Rule::boolean => {
            let value = match pair.as_str() {
                "true" => true,
                "false" => false,
                text => return Err(invalid_literal_error(source, "boolean", text, span)),
            };
            Expr::Bool(value, span)
        }

        Rule::string => {
            let content = unescape_string(pair.as_str())?;
            Expr::String(content, span)
        }

        Rule::symbol => {
            let text = pair.as_str();
            if text.contains('.') {
                let components: Vec<String> = text.split('.').map(String::from).collect();
                if components.iter().any(|c| c.is_empty()) {
                    return Err(invalid_literal_error(source, "path", text, span));
                }
                Expr::Path(Path(components), span)
            } else {
                Expr::Symbol(text.to_string(), span)
            }
        }

        Rule::list | Rule::block => {
            let children: Result<Vec<_>, _> =
                pair.into_inner().map(|p| build_node(p, source)).collect();
            Expr::List(children?, span)
        }

        Rule::quote => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| missing_element_error(source, "expression after quote", span))?;
            let quoted = build_node(inner, source)?;
            Expr::Quote(Box::new(quoted), span)
        }

        Rule::param_list => {
            return build_param_list(pair, source);
        }

        Rule::lambda_form => {
            return build_lambda_form(pair, source);
        }

        Rule::define_form => {
            return build_define_form(pair, source);
        }

        Rule::spread_arg => {
            let symbol_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| missing_element_error(source, "symbol after spread", span))?;
            let symbol = build_node(symbol_pair, source)?;
            Expr::Spread(Box::new(symbol))
        }

        rule => {
            return Err(make_error(
                source,
                ErrorKind::MalformedConstruct {
                    construct: format!("unsupported rule: {:?}", rule),
                },
                span,
            ));
        }
    };

    Ok(Spanned {
        value: expr.into(),
        span,
    })
}

fn build_param_list(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    let span = extract_span(&pair);
    let param_items: Vec<_> = pair
        .into_inner()
        .next()
        .unwrap() // grammar guarantees param_items exists
        .into_inner()
        .collect();

    let mut required = Vec::new();
    let mut rest = None;

    for item in param_items {
        match item.as_rule() {
            Rule::symbol if rest.is_none() => {
                required.push(item.as_str().to_string());
            }
            Rule::spread_arg if rest.is_none() => {
                let symbol = item.into_inner().next().unwrap();
                rest = Some(symbol.as_str().to_string());
            }
            Rule::symbol => {
                return Err(make_error(
                    source,
                    ErrorKind::ParameterOrderViolation {
                        rest_span: to_source_span(extract_span(&item)),
                    },
                    extract_span(&item),
                ));
            }
            _ => {
                return Err(invalid_literal_error(
                    source,
                    "parameter",
                    &format!("{:?}", item.as_rule()),
                    extract_span(&item),
                ));
            }
        }
    }

    let param_list = ParamList {
        required,
        rest,
        span,
    };

    Ok(Spanned {
        value: Expr::ParamList(param_list).into(),
        span,
    })
}

fn build_special_form(
    pair: Pair<Rule>,
    source: &SourceContext,
    form_name: &str,
) -> Result<AstNode, SutraError> {
    let span = extract_span(&pair);
    let mut inner = pair.into_inner();

    let param_list = build_param_list(
        inner
            .next()
            .ok_or_else(|| missing_element_error(source, "parameter list", span))?,
        source,
    )?;

    let body = build_node(
        inner
            .next()
            .ok_or_else(|| missing_element_error(source, "function body", span))?,
        source,
    )?;

    // Create synthetic form symbol - NOTE: This is an architectural smell
    // These should be dedicated AST variants, not artificial lists
    let form_symbol = Spanned {
        value: Expr::Symbol(form_name.to_string(), span).into(),
        span,
    };

    Ok(Spanned {
        value: Expr::List(vec![form_symbol, param_list, body], span).into(),
        span,
    })
}

fn build_lambda_form(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    build_special_form(pair, source, "lambda")
}

fn build_define_form(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    build_special_form(pair, source, "define")
}

// ============================================================================
// UTILITIES
// ============================================================================

fn extract_span(pair: &Pair<Rule>) -> Span {
    Span {
        start: pair.as_span().start(),
        end: pair.as_span().end(),
    }
}

fn invalid_literal_error(
    source: &SourceContext,
    literal_type: &str,
    value: &str,
    span: Span,
) -> SutraError {
    make_error(
        source,
        ErrorKind::InvalidLiteral {
            literal_type: literal_type.into(),
            value: value.into(),
        },
        span,
    )
}

fn missing_element_error(source: &SourceContext, element: &str, span: Span) -> SutraError {
    make_error(
        source,
        ErrorKind::MissingElement {
            element: element.into(),
        },
        span,
    )
}

fn unescape_string(text: &str) -> Result<String, SutraError> {
    // Remove surrounding quotes
    let inner = &text[1..text.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some('"') => result.push('"'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    Ok(result)
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

fn make_error(source: &SourceContext, kind: ErrorKind, span: Span) -> SutraError {
    SutraError {
        kind,
        source_info: crate::errors::SourceInfo {
            source: source.to_named_source(),
            primary_span: to_source_span(span),
            phase: "parsing".to_string(),
        },
        diagnostic_info: crate::errors::DiagnosticInfo {
            help: None,
            error_code: "sutra::parse".to_string(),
        },
    }
}

fn parse_error(error: Error<Rule>, source: &SourceContext) -> SutraError {
    let span = match error.location {
        pest::error::InputLocation::Pos(pos) => Span {
            start: pos,
            end: pos,
        },
        pest::error::InputLocation::Span((start, end)) => Span { start, end },
    };

    // Simple error message improvement
    let message = if error.to_string().contains("expected ')'") {
        "Missing closing parenthesis"
    } else if error.to_string().contains("expected '}'") {
        "Missing closing brace"
    } else if error.to_string().contains("expected '\"'") {
        "Missing closing quote"
    } else {
        "Syntax error"
    };

    make_error(
        source,
        ErrorKind::MalformedConstruct {
            construct: message.to_string(),
        },
        span,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::SourceContext;

    #[test]
    fn test_empty_input() {
        let result = parse("", SourceContext::from_file("test", ""));
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_simple_number() {
        let result = parse("42", SourceContext::from_file("test", "42"));
        assert!(result.is_ok());
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_unmatched_paren() {
        let result = parse("(a b", SourceContext::from_file("test", "(a b"));
        assert!(result.is_err());
    }
}
