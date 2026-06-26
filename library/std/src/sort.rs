/// Sorting — ordering collections
pub struct SortSpec;

impl SortSpec {
    pub const NAME: &str = "Sort";
    pub const OPERATIONS: &[&str] = &[
        "asc",
        "desc",
        "by_key_asc",
        "by_key_desc",
        "is_sorted",
        "min",
        "max",
    ];
}
