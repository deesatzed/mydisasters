use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::filters::{classify_type, matches_type, matches_filename};
use crate::index::load_or_refresh;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub size: u64,
    pub type_label: String,
}

#[derive(Debug, Clone)]
pub struct DirResult {
    pub dir: PathBuf,
    pub files: Vec<FileEntry>,
}

pub struct ScanConfig {
    pub since: Option<SystemTime>,
    pub until: Option<SystemTime>,
    pub type_spec: Option<String>,
    pub filename: Option<String>,
    pub max_depth: Option<usize>,
}

/// Scans `start` using the index cache at `cache_file` (full walk on cache miss/stale/refresh,
/// re-stat-only fast path on cache hit). Returns matching results plus total files scanned.
pub fn scan_with_cache(
    start: &Path,
    config: &ScanConfig,
    cache_file: &Path,
    refresh: bool,
) -> Result<(Vec<DirResult>, u64), String> {
    let (candidates, scanned_count) = load_or_refresh(start, cache_file, refresh)?;

    let mut dir_map: std::collections::BTreeMap<PathBuf, Vec<FileEntry>> =
        std::collections::BTreeMap::new();

    for cached in candidates {
        let path = cached.path;
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        let ext = path.extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_else(|| {
                if name.starts_with('.') { name.clone() } else { String::new() }
            });

        if let Some(ref fname) = config.filename {
            if !matches_filename(&name, fname) {
                continue;
            }
        }
        if let Some(ref tspec) = config.type_spec {
            if !matches_type(&ext, tspec) {
                continue;
            }
        }
        if let Some(ref max_depth) = config.max_depth {
            let depth = path.strip_prefix(start).map(|p| p.components().count()).unwrap_or(0);
            if depth > *max_depth {
                continue;
            }
        }
        if let Some(since) = config.since {
            if cached.mtime < since { continue; }
        }
        if let Some(until) = config.until {
            if cached.mtime > until { continue; }
        }

        let dir = path.parent().unwrap_or(start).to_path_buf();
        let type_label = classify_type(&ext);

        dir_map.entry(dir).or_default().push(FileEntry {
            name,
            path,
            mtime: cached.mtime,
            size: cached.size,
            type_label,
        });
    }

    let results = dir_map.into_iter()
        .map(|(dir, mut files)| {
            files.sort_by_key(|f| std::cmp::Reverse(f.mtime));
            DirResult { dir, files }
        })
        .collect();

    Ok((results, scanned_count))
}

/// Convenience wrapper for callers that don't need cache persistence (e.g. existing tests).
/// Always does a full walk — never reads or writes a real cache file.
pub fn scan(start: &Path, config: &ScanConfig) -> Vec<DirResult> {
    let tmp_cache = std::env::temp_dir().join(format!(
        "dirtrack-scan-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let result = scan_with_cache(start, config, &tmp_cache, true);
    let _ = std::fs::remove_file(&tmp_cache);
    result.map(|(results, _count)| results).unwrap_or_default()
}


