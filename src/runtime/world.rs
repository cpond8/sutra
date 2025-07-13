use crate::ast::value::Value;
use crate::macros;
use crate::runtime::path::Path;
use im::HashMap;
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;

// Using a concrete, seedable PRNG for determinism.
type SmallRng = Xoshiro256StarStar;

// ============================================================================
// WORLD STATE: Data container for Sutra's world
// ============================================================================

#[derive(Clone, Debug)]
pub struct WorldState {
    data: Value,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            data: Value::Map(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        let mut current = &self.data;
        for key in &path.0 {
            let Value::Map(map) = current else { return None };
            let Some(value) = map.get(key.as_str()) else { return None };
            current = value;
        }
        Some(current)
    }

    pub fn set(&self, path: &Path, val: Value) -> Self {
        if path.0.is_empty() {
            return self.clone();
        }
        let new_data = set_recursive(&self.data, &path.0, val);
        Self { data: new_data }
    }

    pub fn del(&self, path: &Path) -> Self {
        if path.0.is_empty() {
            return self.clone();
        }
        let new_data = del_recursive(&self.data, &path.0);
        Self { data: new_data }
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WORLD: Top-level container for all runtime state
// ============================================================================

#[derive(Clone, Debug)]
pub struct World {
    pub state: WorldState,
    pub prng: SmallRng,
    pub macros: crate::macros::MacroEnv,
}

impl World {
    pub fn new() -> Self {
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_entropy(),
            macros: macros::MacroEnv::new(),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            state: WorldState::new(),
            prng: SmallRng::from_seed(seed),
            macros: macros::MacroEnv::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        self.state.get(path)
    }

    pub fn set(&self, path: &Path, val: Value) -> Self {
        Self {
            state: self.state.set(path, val),
            ..self.clone()
        }
    }

    pub fn del(&self, path: &Path) -> Self {
        Self {
            state: self.state.del(path),
            ..self.clone()
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
    }

    pub fn with_macros(self, macros: crate::macros::MacroEnv) -> Self {
        Self { macros, ..self }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// IMMUTABLE HELPERS: set_recursive, del_recursive
// ============================================================================

// Recursive helper for immutable `set`.
fn set_recursive(current: &Value, path_segments: &[String], val: Value) -> Value {
    let Some(key) = path_segments.first() else {
        return current.clone();
    };

    let remaining_segments = &path_segments[1..];
    let mut map = match current {
        Value::Map(m) => m.clone(),
        _ => HashMap::new(),
    };

    if remaining_segments.is_empty() {
        map.insert(key.clone(), val);
    } else {
        let child = map.get(key).unwrap_or(&Value::Nil);
        let new_child = set_recursive(child, remaining_segments, val);
        map.insert(key.clone(), new_child);
    }

    Value::Map(map)
}

// Recursive helper for immutable `del`.
fn del_recursive(current: &Value, path_segments: &[String]) -> Value {
    let Some(key) = path_segments.first() else {
        return current.clone();
    };

    let Value::Map(current_map) = current else {
        return current.clone();
    };

    let mut map = current_map.clone();

    if path_segments.len() == 1 {
        map.remove(key);
    } else if let Some(child) = map.get(key) {
        let new_child = del_recursive(child, &path_segments[1..]);
        if let Value::Map(child_map) = &new_child {
            if child_map.is_empty() {
                map.remove(key);
            } else {
                map.insert(key.clone(), new_child);
            }
        } else {
            map.insert(key.clone(), new_child);
        }
    }

    Value::Map(map)
}

// ============================================================================
// STATE CONTEXT IMPLEMENTATION
// ============================================================================

impl crate::atoms::StateContext for WorldState {
    fn get(&self, path: &crate::runtime::path::Path) -> Option<&crate::ast::value::Value> {
        self.get(path)
    }

    fn set(&mut self, path: &crate::runtime::path::Path, value: crate::ast::value::Value) {
        if path.0.is_empty() {
            return;
        }
        self.data = set_recursive(&self.data, &path.0, value);
    }

    fn del(&mut self, path: &crate::runtime::path::Path) {
        if path.0.is_empty() {
            return;
        }
        self.data = del_recursive(&self.data, &path.0);
    }

    fn exists(&self, path: &crate::runtime::path::Path) -> bool {
        self.get(path).is_some()
    }
}
