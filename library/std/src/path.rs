/// Path manipulation — join, split, extension (Go: path/filepath, Rust: std::path, Python: os.path)
pub struct PathSpec;

impl PathSpec {
    pub const NAME: &str = "Path";
    pub const OPERATIONS: &[&str] = &[
        "join", "basename", "dirname", "extension", "is_absolute",
        "normalize", "relative", "split",
    ];
}
