use crate::value::Value;
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

    pub fn get(&self, path: &[&str]) -> Option<&Value> {
        let mut current = &self.data;
        for key in path {
            if let Value::Map(map) = current {
                if let Some(value) = map.get(*key) {
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

    pub fn set(&self, path: &[&str], val: Value) -> Self {
        if path.is_empty() {
            return self.clone(); // Or handle as an error
        }
        let new_data = set_recursive(&self.data, path, val);
        Self {
            data: new_data,
            prng: self.prng.clone(),
        }
    }

    pub fn del(&self, path: &[&str]) -> Self {
        if path.is_empty() {
            return self.clone(); // Or handle as an error
        }
        let new_data = del_recursive(&self.data, path);
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
fn set_recursive(current: &Value, path: &[&str], val: Value) -> Value {
    // Base case: If path is empty, we've reached the target. Return the new value.
    if path.is_empty() {
        return val;
    }

    let key = path[0];
    let mut map = match current {
        Value::Map(m) => m.clone(),
        // If the current value is not a map, we create a new one to continue the path.
        _ => HashMap::new(),
    };

    // Recurse: Get the child to update, or start with a Nil value if it doesn't exist.
    let child = map.get(key).unwrap_or(&Value::Nil);
    let new_child = set_recursive(child, &path[1..], val);

    // Update the map with the new child and return the new map as a Value.
    map.insert(key.to_string(), new_child);
    Value::Map(map)
}

// Recursive helper for immutable `del`.
fn del_recursive(current: &Value, path: &[&str]) -> Value {
    // If the current value is not a map or the path is empty, we can't delete anything.
    let (map, key) = match (current, path.first()) {
        (Value::Map(m), Some(k)) => (m, *k),
        _ => return current.clone(),
    };

    // If the key doesn't exist in the map, there's nothing to do.
    if !map.contains_key(key) {
        return current.clone();
    }

    let mut new_map = map.clone();

    // Base case: If this is the last key in the path, remove it from the new map.
    if path.len() == 1 {
        new_map.remove(key);
    } else {
        // Recurse: Get the child and apply the deletion to it.
        let child = new_map.get(key).unwrap(); // Safe due to contains_key check
        let new_child = del_recursive(child, &path[1..]);

        // If the child is now an empty map after deletion, remove it from this map.
        // Otherwise, update the map with the modified child.
        if let Value::Map(m) = &new_child {
            if m.is_empty() {
                new_map.remove(key);
            } else {
                new_map.insert(key.to_string(), new_child);
            }
        } else {
            new_map.insert(key.to_string(), new_child);
        }
    }

    Value::Map(new_map)
}
