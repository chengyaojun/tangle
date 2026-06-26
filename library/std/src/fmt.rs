//! fmt — formatted I/O.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | fmt | std::fmt | print() / input() |
//!
//! ## Operations
//! print, println, input, debug, error, format

/// Formatted I/O — print, input, debug (Go: fmt, Rust: std::fmt, Python: print/input)
pub struct FmtSpec;

impl FmtSpec {
    pub const NAME: &str = "fmt";
    pub const OPERATIONS: &[&str] = &["print", "println", "input", "debug", "error", "format"];
}
