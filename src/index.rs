use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

const TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFile {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexCache {
    pub root: PathBuf,
    pub walked_at: SystemTime,
    pub files: Vec<CachedFile>,
}

/// Returns the list of files under `root` plus the total count scanned,
/// using `cache_file` as the persisted index. `force_refresh` bypasses
/// the cache entirely (equivalent to the `--refresh` CLI flag).
pub fn load_or_refresh(
    root: &Path,
    cache_file: &Path,
    force_refresh: bool,
) -> Result<(Vec<CachedFile>, u64), String> {
    if !force_refresh {
        if let Some(cache) = read_cache(cache_file) {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
            if cache.root == canonical_root && is_fresh(&cache) {
                return Ok(restat(cache.files));
            }
        }
    }

    let (files, count) = walk(root);
    write_cache(cache_file, root, &files)?;
    Ok((files, count))
}

/// Computes the cache file path for a given scan root, under the provided base dir
/// (caller passes `~/.config/dirtrack/index`).
pub fn cache_path_for_root(index_dir: &Path, root: &Path) -> PathBuf {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    index_dir.join(format!("{:x}.json", hasher.finish()))
}

fn is_fresh(cache: &IndexCache) -> bool {
    match SystemTime::now().duration_since(cache.walked_at) {
        Ok(age) => age < TTL,
        Err(_) => true, // walked_at is in the future (clock skew) — treat as fresh
    }
}

fn read_cache(cache_file: &Path) -> Option<IndexCache> {
    let raw = std::fs::read_to_string(cache_file).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_cache(cache_file: &Path, root: &Path, files: &[CachedFile]) -> Result<(), String> {
    if let Some(parent) = cache_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create index dir: {}", e))?;
    }
    let cache = IndexCache {
        root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        walked_at: SystemTime::now(),
        files: files.to_vec(),
    };
    let json = serde_json::to_string_pretty(&cache).map_err(|e| format!("serialize error: {}", e))?;
    std::fs::write(cache_file, json).map_err(|e| format!("write error: {}", e))
}

/// Re-stats every cached path; drops any path that no longer exists.
fn restat(cached: Vec<CachedFile>) -> (Vec<CachedFile>, u64) {
    let mut out = Vec::with_capacity(cached.len());
    for entry in cached {
        if let Ok(metadata) = std::fs::metadata(&entry.path) {
            if let Ok(mtime) = metadata.modified() {
                out.push(CachedFile {
                    path: entry.path,
                    mtime,
                    size: metadata.len(),
                });
            }
        }
    }
    let count = out.len() as u64;
    (out, count)
}

const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".next", "__pycache__"];

fn walk(root: &Path) -> (Vec<CachedFile>, u64) {
    let mut files = Vec::new();
    let mut count: u64 = 0;

    let walker = WalkDir::new(root).follow_links(false);
    for entry in walker.into_iter().filter_entry(|e| {
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            return !SKIP_DIRS.contains(&name.as_ref());
        }
        true
    }).filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        count += 1;
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = match metadata.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };
        files.push(CachedFile {
            path: entry.path().to_path_buf(),
            mtime,
            size: metadata.len(),
        });
    }

    (files, count)
}
