pub struct IoSpec;

impl IoSpec {
    pub const NAME: &str = "IO";
    pub const OPERATIONS: &[&str] = &["readFile", "writeFile", "exists", "delete"];
}
