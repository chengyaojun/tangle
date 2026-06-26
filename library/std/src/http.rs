//! HTTP — HTTP client.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | net/http | reqwest / hyper | requests / urllib |
//!
//! ## Operations
//! get, post, put, delete

pub struct HttpSpec;

impl HttpSpec {
    pub const NAME: &str = "HTTP";
    pub const OPERATIONS: &[&str] = &["get", "post", "put", "delete"];
}
