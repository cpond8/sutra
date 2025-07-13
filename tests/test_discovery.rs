// Test discovery, loading, and filtering for Sutra test harness.
use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use walkdir::WalkDir;

/// Represents a single YAML test case for the Sutra test harness.
#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    #[allow(dead_code)]
    pub style: String,
    pub input: String,
    pub expected: Option<String>,
    pub expect_error: Option<String>,
    pub expect_error_code: Option<String>,
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub only: bool,
}

/// Discovers all YAML files recursively under the given root directory.
/// Returns only files with .yaml or .yml extensions.
pub fn discover_yaml_files<P: AsRef<Path>>(root: P) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map(|ext| ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Loads all test cases from a YAML file at the given path.
/// Returns an empty vector if the file cannot be read or parsed, after printing an error.
pub fn load_test_cases(path: &Path) -> Vec<TestCase> {
    match fs::read_to_string(path) {
        Ok(content) => match serde_yaml::from_str::<Vec<TestCase>>(&content) {
            Ok(cases) => cases,
            Err(e) => {
                eprintln!("Failed to parse YAML in {}: {}", path.display(), e);
                Vec::new()
            }
        },
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// Helper for test skipping logic.
pub fn skip_reason(case: &TestCase, has_only: bool, filter: Option<&str>) -> Option<String> {
    if has_only && !case.only {
        return Some("Not marked 'only' in 'only' mode".to_string());
    }
    if case.skip {
        return Some("Marked 'skip'".to_string());
    }
    if let Some(f) = filter {
        if !case.name.to_lowercase().contains(f) {
            return Some(format!("Filtered out by substring: {}", f));
        }
    }
    None
}