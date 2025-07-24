use miette::NamedSource;
use std::sync::Arc;

/// Represents source context for error reporting with explicit hierarchy
/// between real sources (preferred) and fallbacks (tolerated when necessary)
#[derive(Debug, Clone)]
pub struct SourceContext {
    pub name: String,
    pub content: String,
}

impl SourceContext {
    /// Create a source context from real file content
    /// This is the preferred method for error reporting
    pub fn from_file(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }

    /// Create a fallback when real source is unavailable
    /// Use only when real source cannot be obtained
    pub fn fallback(context: &str) -> Self {
        Self {
            name: "fallback".to_string(),
            content: format!("// {}", context),
        }
    }

    /// Convert to NamedSource for use with miette error reporting
    pub fn to_named_source(&self) -> Arc<NamedSource<String>> {
        Arc::new(NamedSource::new(self.name.clone(), self.content.clone()))
    }
}

impl Default for SourceContext {
    fn default() -> Self {
        Self::fallback("default context")
    }
}
