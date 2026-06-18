use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use dirtrack::index::{load_or_refresh, IndexCache};

fn cache_path_for(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("index_cache.json")
}

#[test]
fn test_first_scan_writes_cache() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.env"), b"A=1").unwrap();

    let cache_file = cache_path_for(&tmp);
    let (files, _count) = load_or_refresh(&root, &cache_file, false).unwrap();

    assert_eq!(files.len(), 1);
    assert!(cache_file.exists(), "cache file should be written after first scan");
}

#[test]
fn test_cache_hit_skips_walk_but_restats() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.env"), b"A=1").unwrap();

    let cache_file = cache_path_for(&tmp);
    let (files1, _) = load_or_refresh(&root, &cache_file, false).unwrap();
    let original_mtime = files1[0].mtime;

    // Modify the existing file's content (changes mtime) and add a brand-new file.
    std::thread::sleep(Duration::from_millis(1100));
    fs::write(root.join("a.env"), b"A=2").unwrap();
    fs::write(root.join("b.env"), b"B=1").unwrap();

    let (files2, _) = load_or_refresh(&root, &cache_file, false).unwrap();

    // Re-stat of known file picks up new mtime.
    let a = files2.iter().find(|f| f.path.ends_with("a.env")).unwrap();
    assert!(a.mtime > original_mtime, "cache hit should re-stat known files");

    // New file is NOT discovered on a cache hit (documented trade-off).
    assert!(
        !files2.iter().any(|f| f.path.ends_with("b.env")),
        "cache hit should not discover brand-new files"
    );
}

#[test]
fn test_stale_cache_triggers_full_walk() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.env"), b"A=1").unwrap();

    let cache_file = cache_path_for(&tmp);
    let (_files1, _) = load_or_refresh(&root, &cache_file, false).unwrap();

    fs::write(root.join("b.env"), b"B=1").unwrap();

    // Manually backdate the cache's walked_at past the 24h TTL.
    let raw = fs::read_to_string(&cache_file).unwrap();
    let mut cache: IndexCache = serde_json::from_str(&raw).unwrap();
    cache.walked_at = SystemTime::now() - Duration::from_secs(25 * 3600);
    fs::write(&cache_file, serde_json::to_string_pretty(&cache).unwrap()).unwrap();

    let (files2, _) = load_or_refresh(&root, &cache_file, false).unwrap();
    assert!(
        files2.iter().any(|f| f.path.ends_with("b.env")),
        "stale cache (>24h) should trigger a full walk that discovers new files"
    );
}

#[test]
fn test_refresh_flag_forces_full_walk() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.env"), b"A=1").unwrap();

    let cache_file = cache_path_for(&tmp);
    let (_files1, _) = load_or_refresh(&root, &cache_file, false).unwrap();

    fs::write(root.join("b.env"), b"B=1").unwrap();

    // Cache is fresh (just written), but --refresh should force a walk anyway.
    let (files2, _) = load_or_refresh(&root, &cache_file, true).unwrap();
    assert!(
        files2.iter().any(|f| f.path.ends_with("b.env")),
        "--refresh should force a full walk even with a fresh cache"
    );
}

#[test]
fn test_deleted_cached_file_is_dropped() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    let victim = root.join("a.env");
    fs::write(&victim, b"A=1").unwrap();

    let cache_file = cache_path_for(&tmp);
    let (files1, _) = load_or_refresh(&root, &cache_file, false).unwrap();
    assert_eq!(files1.len(), 1);

    fs::remove_file(&victim).unwrap();

    let (files2, _) = load_or_refresh(&root, &cache_file, false).unwrap();
    assert!(files2.is_empty(), "deleted file should be dropped silently, not error");
}
