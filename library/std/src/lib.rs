//! Tangle Standard Library — cross-host abstraction layer.
//!
//! Each module defines the canonical name and operations for a stdlib type.
//! The compiler's codegen layer maps these to host-specific implementations
//! for JavaScript, Python, and Go.
//!
//! ## Module inventory (23 modules)
//!
//! | Category | Modules |
//! |----------|---------|
//! | Collections | List, Map, Set, Option |
//! | Text | String, Regex, Encoding |
//! | I/O & Formatting | fmt, IO, Env |
//! | System & Files | Path, File, Process |
//! | Network | HTTP, JSON |
//! | Math & Data | Math, Random, Sort |
//! | Concurrency | Task, Channel, Sync |
//! | Time | DateTime |
//! | Crypto | Crypto |

pub mod list;
pub mod map;
pub mod set;
pub mod option;
pub mod string;
pub mod regex;
pub mod encoding;
pub mod fmt;
pub mod io;
pub mod env;
pub mod path;
pub mod file;
pub mod process;
pub mod http;
pub mod json;
pub mod math;
pub mod random;
pub mod sort;
pub mod task;
pub mod channel;
pub mod sync;
pub mod datetime;
pub mod crypto;

pub use list::*;
pub use map::*;
pub use set::*;
pub use option::*;
pub use string::*;
pub use regex::*;
pub use encoding::*;
pub use fmt::*;
pub use io::*;
pub use env::*;
pub use path::*;
pub use file::*;
pub use process::*;
pub use http::*;
pub use json::*;
pub use math::*;
pub use random::*;
pub use sort::*;
pub use task::*;
pub use channel::*;
pub use sync::*;
pub use datetime::*;
pub use crypto::*;

/// All standard library module names
pub const ALL_STDLIB_MODULES: &[&str] = &[
    // Collections
    "List", "Map", "Set", "Option",
    // Text
    "String", "Regex", "Encoding",
    // I/O & Formatting
    "fmt", "IO", "Env",
    // System & Files
    "Path", "File", "Process",
    // Network
    "HTTP", "JSON",
    // Math & Data
    "Math", "Random", "Sort",
    // Concurrency
    "Task", "Channel", "Sync",
    // Time
    "DateTime",
    // Crypto
    "Crypto",
];
