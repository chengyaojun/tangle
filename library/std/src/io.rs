//! IO — file I/O and filesystem operations.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | os | std::fs | os / shutil |
//!
//! ## Operations
//! readFile, writeFile, exists, stat, size, is_dir, is_file, mkdir, read_dir, remove, rename, copy, chmod

/// File I/O & filesystem operations — read, write, metadata, directory listing
/// (Go: os, Rust: std::fs, Python: os/shutil)
pub struct IoSpec;

impl IoSpec {
    pub const NAME: &str = "IO";
    pub const OPERATIONS: &[&str] = &[
        // Basic I/O
        "readFile", "writeFile",
        // Metadata & existence
        "exists", "stat", "size", "is_dir", "is_file",
        // Directory & file manipulation
        "mkdir", "read_dir", "remove", "rename", "copy", "chmod",
    ];
}
