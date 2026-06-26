/// Formatted I/O — print, input, debug (Go: fmt, Rust: std::fmt, Python: print/input)
pub struct FmtSpec;

impl FmtSpec {
    pub const NAME: &str = "fmt";
    pub const OPERATIONS: &[&str] = &["print", "println", "input", "debug", "error", "format"];
}
