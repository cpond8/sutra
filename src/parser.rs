use crate::ast::{Expr, Span};
use crate::error::{SutraError, SutraErrorKind};
use std::iter::Peekable;
use std::str::Chars;

// A minimal placeholder span for now.
fn span(start: usize, end: usize) -> Span {
    Span { start, end }
}

pub fn parse_sexpr(source: &str) -> Result<Expr, SutraError> {
    let mut cursor = Cursor {
        source,
        chars: source.chars().peekable(),
        pos: 0,
    };
    let expr = parse_expr(&mut cursor)?;
    cursor.skip_whitespace();
    if cursor.peek().is_some() {
        return Err(SutraError {
            kind: SutraErrorKind::Parse("Unexpected content after root expression.".to_string()),
            span: Some(span(cursor.pos, source.len())),
        });
    }
    Ok(expr)
}

struct Cursor<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn next(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.pos += ch.len_utf8();
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if c.is_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }
}

fn parse_expr(cursor: &mut Cursor) -> Result<Expr, SutraError> {
    cursor.skip_whitespace();
    match cursor.peek() {
        Some('(') => parse_list(cursor),
        Some('"') => parse_string(cursor),
        Some(_) => parse_atom(cursor),
        None => Err(SutraError {
            kind: SutraErrorKind::Parse("Unexpected end of input.".to_string()),
            span: Some(span(cursor.pos, cursor.pos)),
        }),
    }
}

fn parse_list(cursor: &mut Cursor) -> Result<Expr, SutraError> {
    let start = cursor.pos;
    cursor.next(); // Consume '('
    let mut elements = Vec::new();
    loop {
        cursor.skip_whitespace();
        if let Some(')') = cursor.peek() {
            cursor.next(); // Consume ')'
            return Ok(Expr::List(elements, span(start, cursor.pos)));
        }
        if cursor.peek().is_none() {
            return Err(SutraError {
                kind: SutraErrorKind::Parse("Unclosed list.".to_string()),
                span: Some(span(start, cursor.pos)),
            });
        }
        elements.push(parse_expr(cursor)?);
    }
}

fn parse_string(cursor: &mut Cursor) -> Result<Expr, SutraError> {
    let start = cursor.pos;
    cursor.next(); // Consume '"'
    let mut value = String::new();
    while let Some(c) = cursor.next() {
        if c == '"' {
            return Ok(Expr::String(value, span(start, cursor.pos)));
        }
        // Note: This doesn't handle escaped quotes like \"
        value.push(c);
    }
    Err(SutraError {
        kind: SutraErrorKind::Parse("Unclosed string.".to_string()),
        span: Some(span(start, cursor.pos)),
    })
}

fn parse_atom(cursor: &mut Cursor) -> Result<Expr, SutraError> {
    let start = cursor.pos;
    let mut value = String::new();
    while let Some(&c) = cursor.peek() {
        if c.is_whitespace() || c == ')' || c == '(' {
            break;
        }
        value.push(c);
        cursor.next();
    }

    if value.is_empty() {
        return Err(SutraError {
            kind: SutraErrorKind::Parse("Expected an atom, but found none.".to_string()),
            span: Some(span(start, cursor.pos)),
        });
    }

    if let Ok(n) = value.parse::<f64>() {
        return Ok(Expr::Number(n, span(start, cursor.pos)));
    }

    if value == "true" {
        return Ok(Expr::Bool(true, span(start, cursor.pos)));
    }
    if value == "false" {
        return Ok(Expr::Bool(false, span(start, cursor.pos)));
    }

    Ok(Expr::Symbol(value, span(start, cursor.pos)))
}

// ---
// TODO: Parser Implementation Notes
//
// The current parser is a minimal implementation. To be fully robust,
// the following features are needed:
//
// 1.  **Line and Column Tracking:** The `Span` struct and `Cursor` should
//     track line and column numbers for much better error reporting UX.
//
// 2.  **Escape Sequences:** The `parse_string` function does not handle
//     escaped characters within strings (e.g., `\"`, `\\`, `\n`).
//
// 3.  **Comments:** The parser should support comments (e.g., `;` to end of line)
//     and skip them during parsing.
//
// 4.  **More Robust Atom Parsing:** The `parse_atom` function is simple.
//     It could be improved to handle a wider range of number formats
//     (e.g., integers, scientific notation) more explicitly.
//
// 5.  **Performance:** For very large files, a two-pass approach with a
//     dedicated lexer that produces a token stream first might be more
//     performant and easier to debug than a single-pass char-by-char parser.
// ---
