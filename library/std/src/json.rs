pub struct JsonSpec;

impl JsonSpec {
    pub const NAME: &str = "JSON";
    pub const OPERATIONS: &[&str] = &["parse", "stringify"];
}
