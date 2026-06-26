use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs;
use std::path::Path;

/// Compute a content hash for a source file
pub fn file_fingerprint(path: &Path) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Some(hasher.finish())
}

/// Compute fingerprint from a string source
pub fn source_fingerprint(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Dependency fingerprint: hash of a file + all its imports
pub fn dependency_fingerprint(file: &Path, imports: &[String]) -> Option<u64> {
    let mut hasher = DefaultHasher::new();
    if let Some(fp) = file_fingerprint(file) {
        fp.hash(&mut hasher);
    }
    for import in imports {
        let dep_path = Path::new(import);
        if let Some(fp) = file_fingerprint(dep_path) {
            fp.hash(&mut hasher);
        }
    }
    Some(hasher.finish())
}
