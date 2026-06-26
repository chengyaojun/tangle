//! Tangle Standard Library — cross-host abstraction layer.
//!
//! Each module defines the canonical name and operations for a stdlib type.
//! The compiler's codegen layer maps these to host-specific implementations
//! for JavaScript, Python, and Go.
//!
//! ## Module inventory (17 modules)
//!
//! | Category | Modules |
//! |----------|---------|
//! | Collections | List, Map, Set, Option |
//! | Text | String, Regex, Encoding |
//! | I/O | IO, Console, Env |
//! | Network | HTTP, JSON |
//! | Math | Math, Random, Sort |
//! | Time | DateTime |
//! | Crypto | Crypto |

pub mod list;
pub mod map;
pub mod set;
pub mod option;
pub mod string;
pub mod regex;
pub mod encoding;
pub mod io;
pub mod console;
pub mod env;
pub mod http;
pub mod json;
pub mod math;
pub mod random;
pub mod sort;
pub mod datetime;
pub mod crypto;

pub use list::*;
pub use map::*;
pub use set::*;
pub use option::*;
pub use string::*;
pub use regex::*;
pub use encoding::*;
pub use io::*;
pub use console::*;
pub use env::*;
pub use http::*;
pub use json::*;
pub use math::*;
pub use random::*;
pub use sort::*;
pub use datetime::*;
pub use crypto::*;

/// All standard library module names
pub const ALL_STDLIB_MODULES: &[&str] = &[
    "List", "Map", "Set", "Option",
    "String", "Regex", "Encoding",
    "IO", "Console", "Env",
    "HTTP", "JSON",
    "Math", "Random", "Sort",
    "DateTime",
    "Crypto",
];
