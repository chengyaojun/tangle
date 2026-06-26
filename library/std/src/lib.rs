//! Tangle Standard Library — cross-host abstraction layer.
//!
//! Each module defines the canonical name and operations for a stdlib type.
//! The compiler's codegen layer maps these to host-specific implementations
//! for JavaScript, Python, and Go.
//!
//! ## Module inventory (22 modules)
//!
//! | Category | Modules |
//! |----------|---------|
//! | Collections | List, Map, Set, Option |
//! | Text | String, Regex, Encoding |
//! | I/O & System | IO, fmt, Env, Path, Process |
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
    // I/O & System
    "IO", "fmt", "Env", "Path", "Process",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules_count() {
        assert_eq!(ALL_STDLIB_MODULES.len(), 22, "Expected 22 stdlib modules");
    }

    #[test]
    fn test_module_names_unique() {
        let mut sorted = ALL_STDLIB_MODULES.to_vec();
        sorted.sort();
        let mut seen = std::collections::HashSet::new();
        for name in &sorted {
            assert!(seen.insert(name), "Duplicate module name: {}", name);
        }
    }

    #[test]
    fn test_each_module_has_name_and_ops() {
        // Spot-check a few modules representative of each category
        assert!(!ListSpec::NAME.is_empty());
        assert!(!ListSpec::OPERATIONS.is_empty());

        assert!(!MapSpec::NAME.is_empty());
        assert_eq!(MapSpec::NAME, "Map");

        assert!(!SetSpec::NAME.is_empty());
        assert_eq!(SetSpec::NAME, "Set");

        assert!(!OptionSpec::NAME.is_empty());

        assert!(!StringSpec::NAME.is_empty());
        assert!(!RegexSpec::NAME.is_empty());
        assert!(!EncodingSpec::NAME.is_empty());

        assert!(!IoSpec::NAME.is_empty());
        assert!(!FmtSpec::NAME.is_empty());
        assert!(!EnvSpec::NAME.is_empty());
        assert!(!PathSpec::NAME.is_empty());
        assert!(!ProcessSpec::NAME.is_empty());

        assert!(!HttpSpec::NAME.is_empty());
        assert!(!JsonSpec::NAME.is_empty());

        assert!(!MathSpec::NAME.is_empty());
        assert!(!RandomSpec::NAME.is_empty());
        assert!(!SortSpec::NAME.is_empty());

        assert!(!TaskSpec::NAME.is_empty());
        assert!(!ChannelSpec::NAME.is_empty());
        assert!(!SyncSpec::NAME.is_empty());

        assert!(!DateTimeSpec::NAME.is_empty());
        assert!(!CryptoSpec::NAME.is_empty());
    }

    #[test]
    fn test_all_modules_in_list_have_files() {
        // Each module in ALL_STDLIB_MODULES must have a corresponding spec file
        for name in ALL_STDLIB_MODULES {
            let lowercase = name.to_lowercase();
            assert!(
                module_exists(&lowercase),
                "Module '{}' listed in ALL_STDLIB_MODULES but no file found",
                name
            );
        }
    }

    fn module_exists(name: &str) -> bool {
        // Check that the Rust module source file exists
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(format!("{}.rs", name));
        path.exists()
    }
}
