/// Generic Set<T> — unordered collection of unique elements
pub struct SetSpec;

impl SetSpec {
    pub const NAME: &str = "Set";
    pub const OPERATIONS: &[&str] = &[
        "add",
        "remove",
        "contains",
        "size",
        "union",
        "intersection",
        "difference",
        "to_list",
    ];
}
