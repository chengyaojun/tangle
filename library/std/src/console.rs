/// Console I/O — formatted output and user input
pub struct ConsoleSpec;

impl ConsoleSpec {
    pub const NAME: &str = "Console";
    pub const OPERATIONS: &[&str] = &["print", "println", "input", "debug", "error"];
}
