//! Math — mathematical functions.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | math | std::f64 / num | math |
//!
//! ## Operations
//! abs, min, max, floor, ceil, round, sqrt, pow

pub struct MathSpec;

impl MathSpec {
    pub const NAME: &str = "Math";
    pub const OPERATIONS: &[&str] = &["abs", "min", "max", "floor", "ceil", "round", "sqrt", "pow"];
}
