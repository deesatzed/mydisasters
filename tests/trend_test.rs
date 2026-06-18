use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use dirtrack::trend::{append_entry, deltas_for_root, entries_for_root, TrendEntry};

fn entry(root: &PathBuf, walked_at: SystemTime, file_count: u64, total_size: u64) -> TrendEntry {
    TrendEntry {
        root: root.clone(),
        walked_at,
        file_count,
        total_size,
    }
}

#[test]
fn test_append_and_read_round_trip() {
    let tmp = TempDir::new().unwrap();
    let log_file = tmp.path().join("trend.json");
    let root = tmp.path().join("project");

    let t1 = SystemTime::now() - Duration::from_secs(3600);
    let t2 = SystemTime::now();

    append_entry(&log_file, entry(&root, t1, 10, 1000)).unwrap();
    append_entry(&log_file, entry(&root, t2, 12, 1200)).unwrap();

    let entries = entries_for_root(&log_file, &root);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].file_count, 10);
    assert_eq!(entries[1].file_count, 12);
}

#[test]
fn test_entries_for_root_filters_other_roots() {
    let tmp = TempDir::new().unwrap();
    let log_file = tmp.path().join("trend.json");
    let root_a = tmp.path().join("a");
    let root_b = tmp.path().join("b");

    append_entry(&log_file, entry(&root_a, SystemTime::now(), 5, 500)).unwrap();
    append_entry(&log_file, entry(&root_b, SystemTime::now(), 7, 700)).unwrap();

    let entries = entries_for_root(&log_file, &root_a);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_count, 5);
}

#[test]
fn test_deltas_for_root_computes_correct_differences() {
    let tmp = TempDir::new().unwrap();
    let log_file = tmp.path().join("trend.json");
    let root = tmp.path().join("project");

    let t1 = SystemTime::now() - Duration::from_secs(7200);
    let t2 = SystemTime::now() - Duration::from_secs(3600);
    let t3 = SystemTime::now();

    append_entry(&log_file, entry(&root, t1, 10, 1000)).unwrap();
    append_entry(&log_file, entry(&root, t2, 15, 1500)).unwrap();
    append_entry(&log_file, entry(&root, t3, 12, 900)).unwrap();

    let deltas = deltas_for_root(&log_file, &root);
    assert_eq!(deltas.len(), 2);

    assert_eq!(deltas[0].file_count_delta, 5);
    assert_eq!(deltas[0].total_size_delta, 500);

    assert_eq!(deltas[1].file_count_delta, -3);
    assert_eq!(deltas[1].total_size_delta, -600);
}

#[test]
fn test_deltas_empty_with_fewer_than_two_entries() {
    let tmp = TempDir::new().unwrap();
    let log_file = tmp.path().join("trend.json");
    let root = tmp.path().join("project");

    append_entry(&log_file, entry(&root, SystemTime::now(), 1, 100)).unwrap();

    let deltas = deltas_for_root(&log_file, &root);
    assert!(deltas.is_empty(), "a single entry should produce no deltas");
}

#[test]
fn test_read_log_missing_file_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let log_file = tmp.path().join("does_not_exist.json");
    let root = tmp.path().join("project");

    let entries = entries_for_root(&log_file, &root);
    assert!(entries.is_empty());
}
