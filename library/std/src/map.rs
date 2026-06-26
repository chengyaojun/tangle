//! Map — key-value store.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | map | HashMap | dict |
//!
//! ## Operations
//! get, set, has, keys, values, delete

/// Generic Map<K,V> — key-value store
pub struct MapSpec;

impl MapSpec {
    pub const NAME: &str = "Map";
    pub const OPERATIONS: &[&str] = &["get", "set", "has", "keys", "values", "delete"];
}
