use crate::atoms::{OutputSink, StateContext};
use rand::RngCore;

/// A container for all the services an atom might need during evaluation.
/// This struct provides a clean, type-safe way to pass dependencies to atoms.
pub struct ExecutionContext<'a> {
    pub state: &'a mut dyn StateContext,
    pub output: &'a mut dyn OutputSink,
    pub rng: &'a mut dyn RngCore,
}