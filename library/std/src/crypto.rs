//! Crypto — cryptographic operations.
//!
//! ## Host Mappings
//! | Go | Rust | Python |
//! |----|------|--------|
//! | crypto/* | ring / rust-crypto | hashlib / hmac |
//!
//! ## Operations
//! md5, sha1, sha256, sha512, hmac_sha256, rsa_sign, rsa_verify, aes_encrypt, aes_decrypt

pub struct CryptoSpec;

impl CryptoSpec {
    pub const NAME: &str = "Crypto";
    pub const OPERATIONS: &[&str] = &["md5", "sha1", "sha256", "sha512", "hmac_sha256", "rsa_sign", "rsa_verify", "aes_encrypt", "aes_decrypt"];
}
