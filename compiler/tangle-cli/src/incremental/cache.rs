use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Incremental compilation cache
pub struct IncrementalCache {
    cache_dir: PathBuf,
    /// file path -> fingerprint
    fingerprints: HashMap<String, u64>,
}

impl IncrementalCache {
    pub fn new(cache_dir: &Path) -> Self {
        let _ = fs::create_dir_all(cache_dir);
        let mut cache = IncrementalCache {
            cache_dir: cache_dir.to_path_buf(),
            fingerprints: HashMap::new(),
        };
        cache.load_fingerprints();
        cache
    }

    fn fingerprints_path(&self) -> PathBuf {
        self.cache_dir.join("fingerprints.json")
    }

    fn load_fingerprints(&mut self) {
        if let Ok(data) = fs::read_to_string(self.fingerprints_path()) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, u64>>(&data) {
                self.fingerprints = map;
            }
        }
    }

    fn save_fingerprints(&self) {
        if let Ok(json) = serde_json::to_string(&self.fingerprints) {
            let _ = fs::write(self.fingerprints_path(), json);
        }
    }

    /// Check if a file needs recompilation. Returns true if file changed.
    pub fn needs_recompile(&mut self, file: &str, fingerprint: u64) -> bool {
        let changed = self.fingerprints.get(file) != Some(&fingerprint);
        if changed {
            self.fingerprints.insert(file.to_string(), fingerprint);
            self.save_fingerprints();
        }
        changed
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}
