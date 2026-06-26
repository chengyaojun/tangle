/// Generic List<T> — ordered collection
pub struct ListSpec;

impl ListSpec {
    pub const NAME: &str = "List";
    pub const OPERATIONS: &[&str] = &["length", "map", "filter", "push", "get"];
}
