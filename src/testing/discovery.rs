use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use miette::NamedSource;
use walkdir::WalkDir;

use crate::ast::{AstNode, Expr, Span};
use crate::diagnostics::SutraError;
use crate::syntax::parser;
use crate::{err_msg, err_src};

/// A test definition extracted from a `.sutra` file before macro expansion.
#[derive(Debug, Clone)]
pub struct RawTestDefinition {
    pub name: String,
    pub expect_form: Option<AstNode>,
    pub body: Vec<AstNode>,
    pub span: Span,
    pub source_file: Arc<NamedSource<String>>,
}

/// Discovers tests within a Sutra project.
#[derive(Debug)]
pub struct TestDiscoverer;

impl TestDiscoverer {
    /// Recursively scans a directory for `.sutra` files.
    ///
    /// The returned list of files is sorted to ensure deterministic execution order.
    pub fn discover_test_files<P: AsRef<Path>>(root: P) -> Result<Vec<PathBuf>, SutraError> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root) {
            let entry = entry.map_err(|e| {
                err_msg!(
                    Internal,
                    format!("Failed to walk directory: {}", e)
                )
            })?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "sutra" {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        files.sort();
        Ok(files)
    }

    /// Parses a single `.sutra` file and extracts all `(test ...)` forms.
    ///
    /// This function does not perform macro expansion. It only extracts the raw
    /// test definitions.
    pub fn extract_tests_from_file<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<RawTestDefinition>, SutraError> {
        let path_str = file_path.as_ref().display().to_string();
        let source = fs::read_to_string(file_path.as_ref()).map_err(|e| {
            err_msg!(
                Internal,
                format!("Failed to read file '{}': {}", path_str, e)
            )
        })?;

        let ast = parser::parse(&source)?;
        let source_file = Arc::new(NamedSource::new(path_str, source.clone()));

        let mut tests = Vec::new();
        for node in ast {
            let Expr::List(items, span) = &*node.value else { continue };
            let Some(head) = items.first() else { continue };
            let Expr::Symbol(s, _) = &*head.value else { continue };
            if s != "test" { continue }
            tests.push(Self::parse_test_form(
                items,
                *span,
                source_file.clone(),
            )?);
        }

        Ok(tests)
    }

    fn parse_test_form(
        items: &[AstNode],
        span: Span,
        source_file: Arc<NamedSource<String>>,
    ) -> Result<RawTestDefinition, SutraError> {
        // (test "test-name" (expect ...) body...)
        if items.len() < 2 {
            return Err(err_src!(
                Validation,
                "Invalid test form: expected at least a name",
                source_file.clone(),
                span
            ));
        }

        let name = match &*items[1].value {
            Expr::String(s, _) => s.clone(),
            _ => {
                return Err(err_src!(
                    Validation,
                    "Invalid test form: test name must be a string",
                    source_file.clone(),
                    items[1].span
                ))
            }
        };

        // Try to extract an (expect ...) form as the first body element, if present
        let mut body_start_index = 2;
        let expect_form = match items.get(2) {
            Some(node) => match &*node.value {
                Expr::List(expect_items, _) => match expect_items.first().map(|h| &*h.value) {
                    Some(Expr::Symbol(s, _)) if s == "expect" => {
                        body_start_index = 3;
                        Some(node.clone())
                    }
                    _ => None,
                },
                _ => None,
            },
            None => None,
        };

        let body = items.get(body_start_index..).unwrap_or_default().to_vec();

        Ok(RawTestDefinition {
            name,
            expect_form,
            body,
            span,
            source_file,
        })
    }
}