//! Sync — synchronization primitives.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | sync | std::sync | threading |
//!
//! ## Operations
//! mutex_new, mutex_lock, mutex_unlock, once_do, wait_group_new, wait_group_add, wait_group_done, wait_group_wait

/// Synchronization primitives — mutex, once, wait group
/// (Go: sync, Rust: std::sync, Python: threading)
pub struct SyncSpec;

impl SyncSpec {
    pub const NAME: &str = "Sync";
    pub const OPERATIONS: &[&str] = &[
        "mutex_new", "mutex_lock", "mutex_unlock",
        "once_do", "wait_group_new", "wait_group_add", "wait_group_done", "wait_group_wait",
    ];
}
