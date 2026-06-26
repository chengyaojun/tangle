pub struct StringSpec;

impl StringSpec {
    pub const NAME: &str = "String";
    pub const OPERATIONS: &[&str] = &["length", "concat", "split", "replace", "to_upper", "to_lower", "trim", "contains"];
}
