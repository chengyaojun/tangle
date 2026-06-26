//! Random — random number generation.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | math/rand | rand crate | random |
//!
//! ## Operations
//! int, int_range, float, bool, bytes, shuffle, choice

/// Random number generation — for nonces, test data, sampling
pub struct RandomSpec;

impl RandomSpec {
    pub const NAME: &str = "Random";
    pub const OPERATIONS: &[&str] = &[
        "int",
        "int_range",
        "float",
        "bool",
        "bytes",
        "shuffle",
        "choice",
    ];
}
