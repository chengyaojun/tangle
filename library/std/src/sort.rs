//! Sort — sorting and ordering.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | sort | slice::sort | sorted() / list.sort() |
//!
//! ## Operations
//! asc, desc, by_key_asc, by_key_desc, is_sorted, min, max

/// Sorting — ordering collections
pub struct SortSpec;

impl SortSpec {
    pub const NAME: &str = "Sort";
    pub const OPERATIONS: &[&str] = &[
        "asc",
        "desc",
        "by_key_asc",
        "by_key_desc",
        "is_sorted",
        "min",
        "max",
    ];
}
