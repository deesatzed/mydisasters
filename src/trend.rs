use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::index::CachedFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendEntry {
    pub root: PathBuf,
    pub walked_at: SystemTime,
    pub file_count: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone)]
pub struct TrendDelta {
    pub from: TrendEntry,
    pub to: TrendEntry,
    pub file_count_delta: i64,
    pub total_size_delta: i64,
}

/// Builds a TrendEntry from a freshly-scanned file list (e.g. the index cache
/// contents right after a scan), so callers don't need to re-walk the filesystem.
pub fn build_entry(root: &Path, walked_at: SystemTime, files: &[CachedFile]) -> TrendEntry {
    TrendEntry {
        root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        walked_at,
        file_count: files.len() as u64,
        total_size: files.iter().map(|f| f.size).sum(),
    }
}

/// Appends a new trend entry for `root` to the log at `log_file`, preserving
/// all prior entries (including those for other roots).
pub fn append_entry(log_file: &Path, entry: TrendEntry) -> Result<(), String> {
    let mut entries = read_log(log_file);
    entries.push(entry);

    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create trend dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("serialize error: {}", e))?;
    std::fs::write(log_file, json).map_err(|e| format!("write error: {}", e))
}

/// Reads all trend entries from `log_file`. Returns an empty vec if the file
/// doesn't exist or can't be parsed.
pub fn read_log(log_file: &Path) -> Vec<TrendEntry> {
    std::fs::read_to_string(log_file)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Returns entries for `root` only, in chronological order (oldest first).
pub fn entries_for_root(log_file: &Path, root: &Path) -> Vec<TrendEntry> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut entries: Vec<TrendEntry> = read_log(log_file)
        .into_iter()
        .filter(|e| e.root == canonical)
        .collect();
    entries.sort_by_key(|e| e.walked_at);
    entries
}

/// Computes deltas between consecutive entries for `root` (oldest-to-newest pairs).
pub fn deltas_for_root(log_file: &Path, root: &Path) -> Vec<TrendDelta> {
    let entries = entries_for_root(log_file, root);
    entries
        .windows(2)
        .map(|pair| {
            let (from, to) = (pair[0].clone(), pair[1].clone());
            TrendDelta {
                file_count_delta: to.file_count as i64 - from.file_count as i64,
                total_size_delta: to.total_size as i64 - from.total_size as i64,
                from,
                to,
            }
        })
        .collect()
}
