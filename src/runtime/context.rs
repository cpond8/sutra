use crate::atoms::{StateContext, SharedOutput};
use rand::RngCore;

/// A container for all the services an atom might need during evaluation.
/// This struct provides a clean, type-safe way to pass dependencies to atoms.
pub struct AtomExecutionContext<'a> {
    pub state: &'a mut dyn StateContext,
    pub output: SharedOutput,
    pub rng: &'a mut dyn RngCore,
}