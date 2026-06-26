//! DateTime — date and time operations.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | time | std::time / chrono | datetime |
//!
//! ## Operations
//! now, format, parse, add_days, diff_seconds, timestamp

pub struct DateTimeSpec;

impl DateTimeSpec {
    pub const NAME: &str = "DateTime";
    pub const OPERATIONS: &[&str] = &["now", "format", "parse", "add_days", "diff_seconds", "timestamp"];
}
