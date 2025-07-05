//! A canonical, type-safe representation of a path into the world state.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path(pub Vec<String>);
