//! String — string manipulation.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | strings | std::string::String | str |
//!
//! ## Operations
//! length, concat, split, replace, to_upper, to_lower, trim, contains

pub struct StringSpec;

impl StringSpec {
    pub const NAME: &str = "String";
    pub const OPERATIONS: &[&str] = &["length", "concat", "split", "replace", "to_upper", "to_lower", "trim", "contains"];
}
