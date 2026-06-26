use crate::ir::graph::RuleGraph;
use std::fs;
use std::path::Path;

/// IR cache: persist RuleGraph to disk as JSON
pub struct IrCache {
    cache_dir: std::path::PathBuf,
}

impl IrCache {
    pub fn new(cache_dir: &Path) -> Self {
        let _ = fs::create_dir_all(cache_dir);
        IrCache {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    fn ir_path(&self, file: &str) -> std::path::PathBuf {
        let hash = crate::incremental::fingerprint::source_fingerprint(file);
        self.cache_dir.join(format!("{:016x}.ir.json", hash))
    }

    /// Load cached IR if available
    pub fn load(&self, file: &str) -> Option<RuleGraph> {
        let path = self.ir_path(file);
        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save IR to cache
    pub fn save(&self, file: &str, graph: &RuleGraph) {
        let path = self.ir_path(file);
        if let Ok(json) = serde_json::to_string_pretty(graph) {
            let _ = fs::write(path, json);
        }
    }
}
