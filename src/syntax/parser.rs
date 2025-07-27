//! Sutra Parser - Clean, Minimal Implementation
//!
//! Converts Sutra source code into Abstract Syntax Tree nodes with source location tracking.
//! This parser is purely syntactic - no semantic analysis or type checking.

use crate::errors::{to_source_span, ErrorKind, SourceContext, SutraError};
use crate::{prelude::*, syntax::ParamList};
use pest::{error::Error, iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax/grammar.pest"]
struct SutraParser;

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parse Sutra source code into AST nodes
pub fn parse(source_text: &str, source_context: SourceContext) -> Result<Vec<AstNode>, SutraError> {
    if source_text.trim().is_empty() {
        return Ok(vec![]);
    }

    let pairs = SutraParser::parse(Rule::program, source_text)
        .map_err(|e| convert_parse_error(e, &source_context))?;

    let program = pairs.peek().unwrap(); // pest guarantees program rule exists

    program
        .into_inner()
        .filter(|p| p.as_rule() != Rule::EOI)
        .map(|p| build_ast_node(p, &source_context))
        .collect()
}

/// Wrap multiple AST nodes in a (do ...) form if needed
pub fn wrap_in_do(nodes: Vec<AstNode>) -> AstNode {
    match nodes.len() {
        0 => make_empty_list(),
        1 => nodes.into_iter().next().unwrap(),
        _ => {
            let span = calculate_span(&nodes);
            let do_symbol = make_symbol("do", span);
            let mut items = Vec::with_capacity(nodes.len() + 1);
            items.push(do_symbol);
            items.extend(nodes);
            make_list(items, span)
        }
    }
}

// ============================================================================
// AST BUILDERS
// ============================================================================

fn build_ast_node(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    let span = get_span(&pair);

    match pair.as_rule() {
        Rule::expr => {
            let inner = pair.into_inner().next().unwrap(); // grammar guarantees inner exists
            build_ast_node(inner, source)
        }

        Rule::atom => {
            let inner = pair.into_inner().next().unwrap(); // grammar guarantees inner exists
            build_ast_node(inner, source)
        }

        Rule::number => {
            let text = pair.as_str();
            let value = text.parse::<f64>().map_err(|_| {
                make_error(
                    source,
                    ErrorKind::InvalidLiteral {
                        literal_type: "number".into(),
                        value: text.into(),
                    },
                    span,
                )
            })?;
            Ok(make_number(value, span))
        }

        Rule::boolean => {
            let value = match pair.as_str() {
                "true" => true,
                "false" => false,
                text => {
                    return Err(make_error(
                        source,
                        ErrorKind::InvalidLiteral {
                            literal_type: "boolean".into(),
                            value: text.into(),
                        },
                        span,
                    ))
                }
            };
            Ok(make_boolean(value, span))
        }

        Rule::string => {
            let content = unescape_string(pair.as_str())?;
            Ok(make_string(content, span))
        }

        Rule::symbol => {
            let text = pair.as_str();
            if text.contains('.') {
                let components: Vec<String> = text.split('.').map(String::from).collect();
                if components.iter().any(|c| c.is_empty()) {
                    return Err(make_error(
                        source,
                        ErrorKind::InvalidLiteral {
                            literal_type: "path".into(),
                            value: text.into(),
                        },
                        span,
                    ));
                }
                Ok(make_path(Path(components), span))
            } else {
                Ok(make_symbol(text, span))
            }
        }

        Rule::list | Rule::block => {
            let children: Result<Vec<_>, _> = pair
                .into_inner()
                .map(|p| build_ast_node(p, source))
                .collect();
            Ok(make_list(children?, span))
        }

        Rule::quote => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                make_error(
                    source,
                    ErrorKind::MissingElement {
                        element: "expression after quote".into(),
                    },
                    span,
                )
            })?;
            let quoted = build_ast_node(inner, source)?;
            Ok(make_quote(quoted, span))
        }

        Rule::param_list => build_param_list(pair, source),

        Rule::lambda_form => build_special_form(pair, "lambda", source),

        Rule::define_form => build_special_form(pair, "define", source),

        Rule::spread_arg => {
            let symbol_pair = pair.into_inner().next().ok_or_else(|| {
                make_error(
                    source,
                    ErrorKind::MissingElement {
                        element: "symbol after spread".into(),
                    },
                    span,
                )
            })?;
            let symbol = build_ast_node(symbol_pair, source)?;
            Ok(make_spread(symbol, span))
        }

        rule => Err(make_error(
            source,
            ErrorKind::MalformedConstruct {
                construct: format!("unsupported rule: {:?}", rule),
            },
            span,
        )),
    }
}

fn build_param_list(pair: Pair<Rule>, source: &SourceContext) -> Result<AstNode, SutraError> {
    let span = get_span(&pair);
    let param_items: Vec<_> = pair
        .into_inner()
        .next()
        .unwrap() // grammar guarantees param_items exists
        .into_inner()
        .collect();

    let mut required = Vec::new();
    let mut rest = None;
    let mut found_rest = false;

    for item in param_items {
        match item.as_rule() {
            Rule::symbol if !found_rest => {
                required.push(item.as_str().to_string());
            }
            Rule::spread_arg if !found_rest => {
                let symbol = item.into_inner().next().unwrap();
                rest = Some(symbol.as_str().to_string());
                found_rest = true;
            }
            Rule::symbol => {
                return Err(make_error(
                    source,
                    ErrorKind::ParameterOrderViolation {
                        rest_span: to_source_span(get_span(&item)),
                    },
                    get_span(&item),
                ));
            }
            _ => {
                return Err(make_error(
                    source,
                    ErrorKind::InvalidLiteral {
                        literal_type: "parameter".into(),
                        value: format!("{:?}", item.as_rule()),
                    },
                    get_span(&item),
                ));
            }
        }
    }

    Ok(make_param_list(
        ParamList {
            required,
            rest,
            span,
        },
        span,
    ))
}

fn build_special_form(
    pair: Pair<Rule>,
    form_name: &str,
    source: &SourceContext,
) -> Result<AstNode, SutraError> {
    let span = get_span(&pair);
    let mut inner = pair.into_inner();

    let param_list = build_param_list(
        inner.next().ok_or_else(|| {
            make_error(
                source,
                ErrorKind::MissingElement {
                    element: "parameter list".into(),
                },
                span,
            )
        })?,
        source,
    )?;

    let body = build_ast_node(
        inner.next().ok_or_else(|| {
            make_error(
                source,
                ErrorKind::MissingElement {
                    element: "function body".into(),
                },
                span,
            )
        })?,
        source,
    )?;

    let form_symbol = make_symbol(form_name, span);
    Ok(make_list(vec![form_symbol, param_list, body], span))
}

// ============================================================================
// AST CONSTRUCTORS
// ============================================================================

fn make_list(items: Vec<AstNode>, span: Span) -> AstNode {
    Spanned {
        value: Expr::List(items, span).into(),
        span,
    }
}

fn make_empty_list() -> AstNode {
    let span = Span { start: 0, end: 0 };
    make_list(vec![], span)
}

fn make_symbol(text: &str, span: Span) -> AstNode {
    Spanned {
        value: Expr::Symbol(text.to_string(), span).into(),
        span,
    }
}

fn make_path(path: Path, span: Span) -> AstNode {
    Spanned {
        value: Expr::Path(path, span).into(),
        span,
    }
}

fn make_string(content: String, span: Span) -> AstNode {
    Spanned {
        value: Expr::String(content, span).into(),
        span,
    }
}

fn make_number(value: f64, span: Span) -> AstNode {
    Spanned {
        value: Expr::Number(value, span).into(),
        span,
    }
}

fn make_boolean(value: bool, span: Span) -> AstNode {
    Spanned {
        value: Expr::Bool(value, span).into(),
        span,
    }
}

fn make_quote(expr: AstNode, span: Span) -> AstNode {
    Spanned {
        value: Expr::Quote(Box::new(expr), span).into(),
        span,
    }
}

fn make_spread(expr: AstNode, span: Span) -> AstNode {
    Spanned {
        value: Expr::Spread(Box::new(expr)).into(),
        span,
    }
}

fn make_param_list(params: ParamList, span: Span) -> AstNode {
    Spanned {
        value: Expr::ParamList(params).into(),
        span,
    }
}

// ============================================================================
// UTILITIES
// ============================================================================

fn get_span(pair: &Pair<Rule>) -> Span {
    Span {
        start: pair.as_span().start(),
        end: pair.as_span().end(),
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

fn unescape_string(text: &str) -> Result<String, SutraError> {
    // Remove surrounding quotes
    let inner = &text[1..text.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
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
        } else {
            result.push(ch);
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
            file_context: crate::errors::FileContext::ParseTime {
                parser_state: "parsing".to_string(),
            },
        },
        diagnostic_info: crate::errors::DiagnosticInfo {
            help: None,
            related_spans: vec![],
            error_code: "sutra::parse".to_string(),
            is_warning: false,
        },
    }
}

fn convert_parse_error(error: Error<Rule>, source: &SourceContext) -> SutraError {
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
