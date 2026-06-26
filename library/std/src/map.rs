/// Generic Map<K,V> — key-value store
pub struct MapSpec;

impl MapSpec {
    pub const NAME: &str = "Map";
    pub const OPERATIONS: &[&str] = &["get", "set", "has", "keys", "values", "delete"];
}
