use std::path::{Path, PathBuf};
use std::sync::Arc;

use miette::NamedSource;
use walkdir::WalkDir;

use crate::ast::{AstNode, Expr, Span};
use crate::syntax::parser;
use crate::SutraError;
use crate::{err_msg, err_src};

/// AST representation of a test definition extracted from a `.sutra` file.
/// Stores the test in AST form to avoid redundant parsing and preserve original
/// span information for diagnostics.
#[derive(Debug, Clone)]
pub struct ASTDefinition {
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
    /// Returns true if the given path has a .sutra extension.
    fn is_sutra_file(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "sutra")
    }

    /// Recursively scans a directory for `.sutra` files.
    ///
    /// The returned list of files is sorted to ensure deterministic execution order.
    pub fn discover_test_files<P: AsRef<Path>>(root: P) -> Result<Vec<PathBuf>, SutraError> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root) {
            let entry = entry
                .map_err(|e| err_msg!(Internal, format!("Failed to walk directory: {}", e)))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !Self::is_sutra_file(path) {
                continue;
            }
            files.push(path.to_path_buf());
        }
        files.sort();
        Ok(files)
    }

    /// Parses a single `.sutra` file and extracts all `(test ...)` forms as AST nodes.
    ///
    /// This function does not perform macro expansion. It extracts the test definitions
    /// in AST form and preserves source context for diagnostics.
    pub fn extract_tests_from_file<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<ASTDefinition>, SutraError> {
        let path_str = file_path.as_ref().display().to_string();
        let source = std::fs::read_to_string(file_path.as_ref()).map_err(|e| {
            err_msg!(
                Internal,
                format!("Failed to read file '{}': {}", path_str, e)
            )
        })?;

        let ast = parser::parse(&source)?;
        let source_file = Arc::new(NamedSource::new(path_str, source));

        let mut tests = Vec::new();
        for node in ast {
            let Expr::List(items, span) = &*node.value else {
                continue;
            };
            let Some(head) = items.first() else { continue };
            let Expr::Symbol(s, _) = &*head.value else {
                continue;
            };
            if s != "test" {
                continue;
            }
            tests.push(Self::parse_test_form(items, *span, source_file.clone())?);
        }

        Ok(tests)
    }

    /// Directly extracts tests from a pre-parsed AST
    pub fn extract_tests_from_ast(
        ast: Vec<AstNode>,
        source_file: Arc<NamedSource<String>>,
    ) -> Result<Vec<ASTDefinition>, SutraError> {
        let mut tests = Vec::new();
        for node in ast {
            let Expr::List(items, span) = &*node.value else {
                continue;
            };
            let Some(head) = items.first() else { continue };
            let Expr::Symbol(s, _) = &*head.value else {
                continue;
            };
            if s != "test" {
                continue;
            }
            tests.push(Self::parse_test_form(items, *span, source_file.clone())?);
        }
        Ok(tests)
    }

    fn parse_test_form(
        items: &[AstNode],
        span: Span,
        source_file: Arc<NamedSource<String>>,
    ) -> Result<ASTDefinition, SutraError> {
        // (test "test-name" (expect ...) body...)
        if items.len() < 2 {
            return Err(err_src!(
                Validation,
                "Invalid test form: expected at least a name",
                &source_file,
                span
            ));
        }

        let name = match &*items[1].value {
            Expr::String(s, _) => s.clone(),
            _ => {
                return Err(err_src!(
                    Validation,
                    "Invalid test form: test name must be a string",
                    &source_file,
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

        Ok(ASTDefinition {
            name,
            expect_form,
            body,
            span,
            source_file,
        })
    }
}
