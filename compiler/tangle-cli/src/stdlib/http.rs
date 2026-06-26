pub struct HttpSpec;

impl HttpSpec {
    pub const NAME: &str = "HTTP";
    pub const OPERATIONS: &[&str] = &["get", "post", "put", "delete"];
}
