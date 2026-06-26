/// Filesystem operations — metadata, directory listing, copy, permissions
/// (Go: os file ops, Rust: std::fs, Python: os/shutil)
pub struct FileSpec;

impl FileSpec {
    pub const NAME: &str = "File";
    pub const OPERATIONS: &[&str] = &[
        "stat", "mkdir", "read_dir", "remove", "rename",
        "copy", "chmod", "size", "is_dir", "is_file",
    ];
}
