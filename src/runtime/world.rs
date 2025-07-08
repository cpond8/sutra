use crate::ast::value::Value;
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
}

impl World {
    pub fn new() -> Self {
        Self {
            data: Value::Map(HashMap::new()),
            prng: SmallRng::from_entropy(),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            data: Value::Map(HashMap::new()),
            prng: SmallRng::from_seed(seed),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Value> {
        let mut current = &self.data;
        for key in &path.0 {
            if let Value::Map(map) = current {
                if let Some(value) = map.get(key.as_str()) {
                    current = value;
                } else {
                    return None;
                }
            } else {
                return None;
            }
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
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.prng.next_u32()
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
    let key = &path_segments[0];
    let remaining_segments = &path_segments[1..];

    let mut map = match current {
        Value::Map(m) => m.clone(),
        // If we're trying to set a value on a non-map, we start with a fresh map.
        _ => HashMap::new(),
    };

    if remaining_segments.is_empty() {
        // Base case: we've reached the end of the path, so insert the value.
        map.insert(key.clone(), val);
    } else {
        // Recursive step: get the child, or a Nil default, and recurse.
        let child = map.get(key).unwrap_or(&Value::Nil);
        let new_child = set_recursive(child, remaining_segments, val);
        map.insert(key.clone(), new_child);
    }
    Value::Map(map)
}

// Recursive helper for immutable `del`.
fn del_recursive(current: &Value, path_segments: &[String]) -> Value {
    let key = if let Some(k) = path_segments.first() {
        k
    } else {
        // Should not happen if called from `del` which checks for empty path.
        return current.clone();
    };

    let mut map = if let Value::Map(m) = current {
        m.clone()
    } else {
        // Cannot delete from a non-map value.
        return current.clone();
    };

    // Base case: If this is the last segment, remove the key and we're done.
    if path_segments.len() == 1 {
        map.remove(key);
        return Value::Map(map);
    }

    // Recursive step: If the child exists, recurse on it.
    if let Some(child) = map.get(key) {
        let new_child = del_recursive(child, &path_segments[1..]);

        // If the recursion resulted in an empty map, remove the key from the current map.
        // Otherwise, update the map with the new, modified child.
        if let Value::Map(ref m) = new_child {
            if m.is_empty() {
                map.remove(key);
            } else {
                map.insert(key.clone(), new_child);
            }
        } else {
            // If the new child is not a map (e.g., Nil), update it.
            map.insert(key.clone(), new_child);
        }
    }

    Value::Map(map)
}
