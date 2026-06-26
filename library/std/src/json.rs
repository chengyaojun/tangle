//! JSON — JSON serialization.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | encoding/json | serde_json | json |
//!
//! ## Operations
//! parse, stringify

pub struct JsonSpec;

impl JsonSpec {
    pub const NAME: &str = "JSON";
    pub const OPERATIONS: &[&str] = &["parse", "stringify"];
}
