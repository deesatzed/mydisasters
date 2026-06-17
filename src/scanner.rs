use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;
use crate::filters::{matches_type, matches_filename};

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

pub fn scan(start: &Path, config: &ScanConfig) -> Vec<DirResult> {
    let mut walker = WalkDir::new(start).follow_links(false);
    if let Some(d) = config.max_depth {
        walker = walker.max_depth(d);
    }

    let mut dir_map: std::collections::BTreeMap<PathBuf, Vec<FileEntry>> =
        std::collections::BTreeMap::new();

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();
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

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = match metadata.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Some(since) = config.since {
            if mtime < since { continue; }
        }
        if let Some(until) = config.until {
            if mtime > until { continue; }
        }

        let dir = path.parent().unwrap_or(start).to_path_buf();
        let type_label = classify_type(&ext);

        dir_map.entry(dir).or_default().push(FileEntry {
            name,
            path,
            mtime,
            size: metadata.len(),
            type_label,
        });
    }

    dir_map.into_iter()
        .map(|(dir, mut files)| {
            files.sort_by(|a, b| b.mtime.cmp(&a.mtime));
            DirResult { dir, files }
        })
        .collect()
}

fn classify_type(ext: &str) -> String {
    match ext {
        e if [".env", ".key", ".pem", ".p12", ".pfx", ".secret"].contains(&e) => "secrets".to_string(),
        e if [".yaml", ".yml", ".toml", ".json", ".ini", ".conf"].contains(&e) => "configs".to_string(),
        e if [".rs", ".ts", ".tsx", ".py", ".go", ".js"].contains(&e) => "code".to_string(),
        _ => "other".to_string(),
    }
}
