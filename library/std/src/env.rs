//! Env — environment variables and process info.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | os | std::env | os.environ / sys |
//!
//! ## Operations
//! get, set, remove, args, current_dir, exit

/// Environment — configuration from environment variables and CLI args
pub struct EnvSpec;

impl EnvSpec {
    pub const NAME: &str = "Env";
    pub const OPERATIONS: &[&str] = &[
        "get",
        "set",
        "remove",
        "args",
        "current_dir",
        "exit",
    ];
}
