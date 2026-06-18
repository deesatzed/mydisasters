use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::index::{read_cache, CachedFile};

#[derive(Debug, Clone)]
pub struct FindMatch {
    pub root: PathBuf,
    pub file: CachedFile,
    pub cache_age: std::time::Duration,
}

/// Case-insensitive substring match, distinct from filters::matches_filename
/// (which is exact-match and used by the existing --file flag).
pub fn matches_fuzzy(filename: &str, pattern: &str) -> bool {
    filename.to_lowercase().contains(&pattern.to_lowercase())
}

/// Searches every persisted index cache under `index_dir` for filenames containing
/// `pattern` (case-insensitive substring). Reads only — never triggers a filesystem walk,
/// so results reflect each root's last scan, however old.
pub fn find_across_caches(index_dir: &Path, pattern: &str, now: SystemTime) -> Vec<FindMatch> {
    let mut matches = Vec::new();

    let entries = match std::fs::read_dir(index_dir) {
        Ok(e) => e,
        Err(_) => return matches,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let cache = match read_cache(&path) {
            Some(c) => c,
            None => continue,
        };
        let cache_age = now.duration_since(cache.walked_at).unwrap_or_default();

        for file in &cache.files {
            let name = match file.path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            if matches_fuzzy(&name, pattern) {
                matches.push(FindMatch {
                    root: cache.root.clone(),
                    file: file.clone(),
                    cache_age,
                });
            }
        }
    }

    matches
}
