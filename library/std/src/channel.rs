//! Channel — typed message passing channels.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | chan | std::sync::mpsc | queue.Queue |
//!
//! ## Operations
//! new, send, recv, close, len, cap, select, try_send, try_recv

/// Typed message passing channels — Go-style channel communication
/// (Go: chan, Rust: std::sync::mpsc, Python: queue.Queue)
pub struct ChannelSpec;

impl ChannelSpec {
    pub const NAME: &str = "Channel";
    pub const OPERATIONS: &[&str] = &[
        "new", "send", "recv", "close", "len", "cap",
        "select", "try_send", "try_recv",
    ];
}
