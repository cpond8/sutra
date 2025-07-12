use crate::ast::value::Value;
use crate::macros;
use crate::runtime::path::Path;
use im::HashMap;
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;

// Using a concrete, seedable PRNG for determinism.
type SmallRng = Xoshiro256StarStar;

#[derive(Clone)]
pub struct World {
    data: Value,
    prng: SmallRng,
    pub macros: crate::macros::MacroEnv,
}

impl World {
    pub fn new() -> Self {
        Self {
            data: Value::Map(HashMap::new()),
            prng: SmallRng::from_entropy(),
            macros: macros::MacroEnv::new(),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            data: Value::Map(HashMap::new()),
            prng: SmallRng::from_seed(seed),
            macros: macros::MacroEnv::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        let mut current = &self.data;
        for key in &path.0 {
            // Guard clause: ensure current is a map
            let Value::Map(map) = current else {
                return None;
            };

            // Guard clause: ensure key exists
            let Some(value) = map.get(key.as_str()) else {
                return None;
            };

            current = value;
        }
        Some(current)
    }

    pub fn set(&self, path: &Path, val: Value) -> Self {
        if path.0.is_empty() {
            return self.clone(); // Or handle as an error
        }
        let new_data = set_recursive(&self.data, &path.0, val);
        Self {
            data: new_data,
            prng: self.prng.clone(),
            macros: self.macros.clone(),
        }
    }

    pub fn del(&self, path: &Path) -> Self {
        if path.0.is_empty() {
            return self.clone(); // Or handle as an error
        }
        let new_data = del_recursive(&self.data, &path.0);
        Self {
            data: new_data,
            prng: self.prng.clone(),
            macros: self.macros.clone(),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
    }

    pub fn with_macros(self, macros: crate::macros::MacroEnv) -> Self {
        Self {
            data: self.data,
            prng: self.prng,
            macros,
        }
    }
}

// Default implementation for convenience
impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

// Recursive helper for immutable `set`.
fn set_recursive(current: &Value, path_segments: &[String], val: Value) -> Value {
    // Guard clause: ensure we have path segments (safety check)
    let Some(key) = path_segments.first() else {
        // Should not happen if called from `set` which checks for empty path.
        return current.clone();
    };

    let remaining_segments = &path_segments[1..];

    let mut map = match current {
        Value::Map(m) => m.clone(),
        // If we're trying to set a value on a non-map, we start with a fresh map.
        _ => HashMap::new(),
    };

    // Base case: we've reached the end of the path, so insert the value and return
    if remaining_segments.is_empty() {
        map.insert(key.clone(), val);
        return Value::Map(map);
    }

    // Recursive step: get the child, or a Nil default, and recurse.
    let child = map.get(key).unwrap_or(&Value::Nil);
    let new_child = set_recursive(child, remaining_segments, val);
    map.insert(key.clone(), new_child);

    Value::Map(map)
}

// Recursive helper for immutable `del`.
fn del_recursive(current: &Value, path_segments: &[String]) -> Value {
    // Guard clause: ensure we have a path segment
    let Some(key) = path_segments.first() else {
        // Should not happen if called from `del` which checks for empty path.
        return current.clone();
    };

    // Guard clause: ensure we're working with a map
    let Value::Map(map) = current else {
        // Cannot delete from a non-map value.
        return current.clone();
    };

    let mut map = map.clone();

    // Base case: If this is the last segment, remove the key and we're done.
    if path_segments.len() == 1 {
        map.remove(key);
        return Value::Map(map);
    }

    // Recursive step: Early return if child doesn't exist
    let Some(child) = map.get(key) else {
        return Value::Map(map);
    };

    let new_child = del_recursive(child, &path_segments[1..]);

    // Handle the child update with guard clause pattern
    match &new_child {
        Value::Map(child_map) if child_map.is_empty() => {
            // Remove empty maps
            map.remove(key);
        }
        _ => {
            // Update with the new child (whether map or other value)
            map.insert(key.clone(), new_child);
        }
    }

    Value::Map(map)
}

// ============================================================================
// STATE CONTEXT IMPLEMENTATION
// ============================================================================

impl crate::atoms::StateContext for World {
    fn get_value(&self, path: &crate::runtime::path::Path) -> Option<crate::ast::value::Value> {
        self.get(path).cloned()
    }

    fn set_value(&mut self, path: &crate::runtime::path::Path, value: crate::ast::value::Value) {
        *self = self.set(path, value);
    }

    fn delete_value(&mut self, path: &crate::runtime::path::Path) {
        *self = self.del(path);
    }

    fn exists(&self, path: &crate::runtime::path::Path) -> bool {
        self.get(path).is_some()
    }
}
