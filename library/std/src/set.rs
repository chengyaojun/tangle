//! Set — unordered unique collection.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | map\[K\]bool / sets | HashSet | set |
//!
//! ## Operations
//! add, remove, contains, size, union, intersection, difference, to_list

/// Generic `Set<T>` — unordered collection of unique elements
pub struct SetSpec;

impl SetSpec {
    pub const NAME: &str = "Set";
    pub const OPERATIONS: &[&str] = &[
        "add",
        "remove",
        "contains",
        "size",
        "union",
        "intersection",
        "difference",
        "to_list",
    ];
}
