/// Random number generation — for nonces, test data, sampling
pub struct RandomSpec;

impl RandomSpec {
    pub const NAME: &str = "Random";
    pub const OPERATIONS: &[&str] = &[
        "int",
        "int_range",
        "float",
        "bool",
        "bytes",
        "shuffle",
        "choice",
    ];
}
