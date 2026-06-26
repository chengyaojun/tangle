/// Binary-to-text encoding — hex and base64
pub struct EncodingSpec;

impl EncodingSpec {
    pub const NAME: &str = "Encoding";
    pub const OPERATIONS: &[&str] = &[
        "hex_encode",
        "hex_decode",
        "base64_encode",
        "base64_decode",
        "url_encode",
        "url_decode",
    ];
}
