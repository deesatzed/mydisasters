use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use dirtrack::scanner::{scan, ScanConfig};

#[test]
fn test_scan_finds_recent_file() {
    let tmp = TempDir::new().unwrap();
    let subdir = tmp.path().join("project/src");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join(".env"), b"SECRET=x").unwrap();

    let config = ScanConfig {
        since: Some(SystemTime::now() - Duration::from_secs(60)),
        until: None,
        type_spec: Some("secrets".to_string()),
        filename: None,
        max_depth: None,
    };

    let results = scan(tmp.path(), &config);
    assert!(!results.is_empty(), "should find the .env file");
    assert!(results.iter().any(|r| r.dir.ends_with("src")));
}

#[test]
fn test_scan_excludes_old_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("old.env"), b"OLD=1").unwrap();

    let config = ScanConfig {
        since: Some(SystemTime::now() + Duration::from_secs(3600)),
        until: None,
        type_spec: Some("secrets".to_string()),
        filename: None,
        max_depth: None,
    };

    let results = scan(tmp.path(), &config);
    assert!(results.is_empty(), "file older than cutoff should be excluded");
}

#[test]
fn test_scan_max_depth() {
    let tmp = TempDir::new().unwrap();
    let deep = tmp.path().join("a/b/c/d");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("file.rs"), b"fn main() {}").unwrap();

    let config = ScanConfig {
        since: None,
        until: None,
        type_spec: Some("code".to_string()),
        filename: None,
        max_depth: Some(2),
    };

    let results = scan(tmp.path(), &config);
    assert!(results.is_empty(), "file beyond max_depth should not appear");
}

#[test]
fn test_scan_specific_filename() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".env"), b"A=1").unwrap();
    fs::write(tmp.path().join(".env.local"), b"B=2").unwrap();

    let config = ScanConfig {
        since: None,
        until: None,
        type_spec: None,
        filename: Some(".env".to_string()),
        max_depth: None,
    };

    let results = scan(tmp.path(), &config);
    assert_eq!(results.len(), 1);
    assert!(results[0].files[0].name == ".env");
}
