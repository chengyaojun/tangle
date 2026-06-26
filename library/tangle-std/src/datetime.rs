pub struct DateTimeSpec;

impl DateTimeSpec {
    pub const NAME: &str = "DateTime";
    pub const OPERATIONS: &[&str] = &["now", "format", "parse", "add_days", "diff_seconds", "timestamp"];
}
