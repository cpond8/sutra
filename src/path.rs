//! A canonical, type-safe representation of a path into the world state.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub Vec<String>);
