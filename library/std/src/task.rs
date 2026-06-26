//! Task — async task operations.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | goroutines | tokio / std::future | asyncio |
//!
//! ## Operations
//! spawn, await, sleep, join, parallel, race, all, timeout

/// Async task operations — spawn, await, sleep for concurrent execution
/// (Go: goroutines, Rust: tokio/std::future, Python: asyncio)
pub struct TaskSpec;

impl TaskSpec {
    pub const NAME: &str = "Task";
    pub const OPERATIONS: &[&str] = &[
        "spawn", "await", "sleep", "join", "parallel",
        "race", "all", "timeout",
    ];
}
