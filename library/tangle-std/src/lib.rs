//! Tangle Standard Library — cross-host abstraction layer.
//!
//! Each module defines the canonical name and operations for a stdlib type.
//! The compiler's codegen layer maps these to host-specific implementations
//! for JavaScript, Python, and Go.

pub mod list;
pub mod map;
pub mod option;
pub mod string;
pub mod json;
pub mod http;
pub mod io;
pub mod math;
pub mod datetime;
pub mod regex;
pub mod crypto;

pub use list::*;
pub use map::*;
pub use option::*;
pub use string::*;
pub use json::*;
pub use http::*;
pub use io::*;
pub use math::*;
pub use datetime::*;
pub use regex::*;
pub use crypto::*;

/// All standard library module names
pub const ALL_STDLIB_MODULES: &[&str] = &[
    "List", "Map", "Option", "String", "JSON", "HTTP", "IO",
    "Math", "DateTime", "Regex", "Crypto",
];
