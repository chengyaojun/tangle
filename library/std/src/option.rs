//! Option — value that may or may not exist.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | — (nil pointers) | Option | None / Optional |
//!
//! ## Operations
//! unwrap, is_some, is_none, map, or_else

/// `Option<T>` — value that may or may not exist
pub struct OptionSpec;

impl OptionSpec {
    pub const NAME: &str = "Option";
    pub const CONSTRUCTORS: &[&str] = &["Some", "None"];
    pub const OPERATIONS: &[&str] = &["unwrap", "is_some", "is_none", "map", "or_else"];
}
