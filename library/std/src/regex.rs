//! Regex — regular expression matching.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | regexp | regex crate | re |
//!
//! ## Operations
//! match, replace, split, test

pub struct RegexSpec;

impl RegexSpec {
    pub const NAME: &str = "Regex";
    pub const OPERATIONS: &[&str] = &["match", "replace", "split", "test"];
}
