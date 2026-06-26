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
