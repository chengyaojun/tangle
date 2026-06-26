//! Process — external command execution.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | os/exec | std::process::Command | subprocess |
//!
//! ## Operations
//! run, exec, spawn, exit, pid, args, stdout, stderr, status

/// Subprocess execution — run external commands
/// (Go: os/exec, Rust: std::process::Command, Python: subprocess)
pub struct ProcessSpec;

impl ProcessSpec {
    pub const NAME: &str = "Process";
    pub const OPERATIONS: &[&str] = &[
        "run", "exec", "spawn", "exit", "pid", "args", "stdout", "stderr", "status",
    ];
}
