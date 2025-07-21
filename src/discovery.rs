use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use miette::NamedSource;
use walkdir::WalkDir;

use crate::prelude::*;
use crate::syntax::parser;

// =====================
// Type Aliases for Complex Types
// =====================

/// Source file context for diagnostics
pub type SourceFile = Arc<NamedSource<String>>;

/// Result of test form extraction - either a valid test or None
pub type TestFormResult = Result<Option<ASTDefinition>, SutraError>;

/// Result of parsing test form structure components
pub type TestFormComponents = (Option<AstNode>, Vec<AstNode>);

/// AST representation of a test definition extracted from a `.sutra` file.
/// Stores the test in AST form to avoid redundant parsing and preserve original
/// span information for diagnostics.
#[derive(Debug, Clone)]
pub struct ASTDefinition {
    pub name: String,
    pub expect_form: Option<AstNode>,
    pub body: Vec<AstNode>,
    pub span: Span,
    pub source_file: SourceFile,
}

/// Discovers and extracts test definitions from Sutra files.
///
/// The discovery process follows this flow:
/// 1. Scan directories for `.sutra` files
/// 2. Parse each file into AST nodes
/// 3. Extract `(test ...)` forms from the AST
/// 4. Validate and structure test definitions
#[derive(Debug)]
pub struct TestDiscoverer;

impl TestDiscoverer {
    // =====================
    // Public API - File Discovery
    // =====================

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

        Self::extract_tests_from_ast(ast, source_file)
    }

    /// Directly extracts tests from a pre-parsed AST
    pub fn extract_tests_from_ast(
        ast: Vec<AstNode>,
        source_file: SourceFile,
    ) -> Result<Vec<ASTDefinition>, SutraError> {
        let mut tests = Vec::new();
        for node in ast {
            let test_form = Self::validate_and_extract_test_form(node, &source_file)?;
            if let Some(test_form) = test_form {
                tests.push(test_form);
            }
        }
        Ok(tests)
    }

    // =====================
    // Internal - Test Form Validation
    // =====================

    /// Validates that a node represents a test form and extracts it if valid.
    ///
    /// A valid test form must:
    /// 1. Be a list expression
    /// 2. Have a symbol "test" as the first element
    /// 3. Have at least a name as the second element
    fn validate_and_extract_test_form(node: AstNode, source_file: &SourceFile) -> TestFormResult {
        let Expr::List(items, span) = &*node.value else {
            return Ok(None);
        };

        let Some(head) = items.first() else {
            return Ok(None);
        };

        let Expr::Symbol(s, _) = &*head.value else {
            return Ok(None);
        };

        if s != "test" {
            return Ok(None);
        }

        let test_form = Self::parse_test_form_structure(items, *span, source_file.clone())?;
        Ok(Some(test_form))
    }

    // =====================
    // Internal - Test Form Parsing
    // =====================

    /// Parses the structure of a test form: `(test "name" (expect ...) body...)`
    fn parse_test_form_structure(
        items: &[AstNode],
        span: Span,
        source_file: SourceFile,
    ) -> Result<ASTDefinition, SutraError> {
        if items.len() < 2 {
            return Err(err_src!(
                Validation,
                "Invalid test form: expected at least a name",
                &source_file,
                span
            ));
        }

        let name = Self::extract_and_validate_test_name(&items[1], &source_file)?;
        let (expect_form, body) = Self::extract_expect_form_and_body(items)?;

        Ok(ASTDefinition {
            name,
            expect_form,
            body,
            span,
            source_file,
        })
    }

    /// Extracts and validates the test name from the second element.
    ///
    /// The name must be a string literal.
    fn extract_and_validate_test_name(
        name_node: &AstNode,
        source_file: &SourceFile,
    ) -> Result<String, SutraError> {
        let Expr::String(s, _) = &*name_node.value else {
            return Err(err_src!(
                Validation,
                "Invalid test form: test name must be a string",
                source_file,
                name_node.span
            ));
        };
        Ok(s.clone())
    }

    /// Extracts the optional expect form and remaining body elements.
    ///
    /// Test forms can have two structures:
    /// - `(test "name" body...)` - no expect form
    /// - `(test "name" (expect ...) body...)` - with expect form
    fn extract_expect_form_and_body(items: &[AstNode]) -> Result<TestFormComponents, SutraError> {
        let Some(second_item) = items.get(2) else {
            let body = items.get(2..).unwrap_or_default().to_vec();
            return Ok((None, body));
        };

        let expect_form = Self::try_extract_expect_form(second_item)?;
        if let Some(expect_form) = expect_form {
            let body = items.get(3..).unwrap_or_default().to_vec();
            return Ok((Some(expect_form), body));
        }

        let body = items.get(2..).unwrap_or_default().to_vec();
        Ok((None, body))
    }

    /// Attempts to extract an expect form from a node.
    ///
    /// An expect form must be a list with "expect" as the first symbol.
    fn try_extract_expect_form(node: &AstNode) -> Result<Option<AstNode>, SutraError> {
        let Expr::List(expect_items, _) = &*node.value else {
            return Ok(None);
        };

        let Some(head) = expect_items.first() else {
            return Ok(None);
        };

        let Expr::Symbol(s, _) = &*head.value else {
            return Ok(None);
        };

        if s != "expect" {
            return Ok(None);
        }

        Ok(Some(node.clone()))
    }

    // =====================
    // Internal - File System Utilities
    // =====================

    /// Returns true if the given path has a .sutra extension.
    fn is_sutra_file(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "sutra")
    }
}
