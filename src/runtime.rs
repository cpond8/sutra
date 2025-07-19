pub mod eval;
pub mod world;

// Re-exports for concise imports
pub use world::{build_canonical_macro_env, build_default_atom_registry};
