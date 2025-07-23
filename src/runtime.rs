pub mod eval;
pub mod world;
pub mod source;

// Re-exports for concise imports
pub use world::build_canonical_macro_env;
pub use source::SourceContext;
