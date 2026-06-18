use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use dirtrack::aggregate::{find_across_caches, matches_fuzzy};
use dirtrack::index::load_or_refresh;

#[test]
fn test_matches_fuzzy_is_case_insensitive_substring() {
    assert!(matches_fuzzy("dram-quest-config.json", "DRAM"));
    assert!(matches_fuzzy(".env.local", "env"));
    assert!(!matches_fuzzy("readme.md", "dram"));
}

#[test]
fn test_find_across_caches_searches_multiple_roots() {
    let tmp = TempDir::new().unwrap();
    let index_dir = tmp.path().join("index");

    let root_a = tmp.path().join("dram-quest");
    fs::create_dir_all(&root_a).unwrap();
    fs::write(root_a.join(".env"), b"A=1").unwrap();
    let cache_a = index_dir.join("a.json");
    load_or_refresh(&root_a, &cache_a, false).unwrap();

    let root_b = tmp.path().join("ersatz-rag");
    fs::create_dir_all(&root_b).unwrap();
    fs::write(root_b.join(".env.production"), b"B=1").unwrap();
    let cache_b = index_dir.join("b.json");
    load_or_refresh(&root_b, &cache_b, false).unwrap();

    let matches = find_across_caches(&index_dir, "env", SystemTime::now());

    assert_eq!(matches.len(), 2, "should find matches across both roots");
    let roots: Vec<_> = matches.iter().map(|m| m.root.clone()).collect();
    assert!(roots.iter().any(|r| r.ends_with("dram-quest")));
    assert!(roots.iter().any(|r| r.ends_with("ersatz-rag")));
}

#[test]
fn test_find_across_caches_no_match_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let index_dir = tmp.path().join("index");

    let root = tmp.path().join("project");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("readme.md"), b"hello").unwrap();
    let cache = index_dir.join("a.json");
    load_or_refresh(&root, &cache, false).unwrap();

    let matches = find_across_caches(&index_dir, "nonexistent-pattern", SystemTime::now());
    assert!(matches.is_empty());
}

#[test]
fn test_find_across_caches_reports_cache_age() {
    let tmp = TempDir::new().unwrap();
    let index_dir = tmp.path().join("index");

    let root = tmp.path().join("project");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".env"), b"A=1").unwrap();
    let cache = index_dir.join("a.json");
    load_or_refresh(&root, &cache, false).unwrap();

    let later = SystemTime::now() + Duration::from_secs(3 * 86400);
    let matches = find_across_caches(&index_dir, "env", later);

    assert_eq!(matches.len(), 1);
    assert!(matches[0].cache_age >= Duration::from_secs(3 * 86400 - 5));
}

#[test]
fn test_find_across_caches_empty_index_dir_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let index_dir = tmp.path().join("does_not_exist");

    let matches = find_across_caches(&index_dir, "anything", SystemTime::now());
    assert!(matches.is_empty());
}
