use dirtrack::output::{build_open_choices, build_summary_groups, build_verbose_groups, format_relative_time, format_summary_line};
use dirtrack::scanner::{DirResult, FileEntry};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[test]
fn test_relative_time_minutes() {
    let t = SystemTime::now() - Duration::from_secs(90);
    assert_eq!(format_relative_time(t), "1m ago");
}

#[test]
fn test_relative_time_hours() {
    let t = SystemTime::now() - Duration::from_secs(7300);
    assert_eq!(format_relative_time(t), "2h ago");
}

#[test]
fn test_relative_time_days() {
    let t = SystemTime::now() - Duration::from_secs(86400 * 3 + 100);
    assert_eq!(format_relative_time(t), "3d ago");
}

#[test]
fn test_summary_line_format() {
    let t = SystemTime::now() - Duration::from_secs(3700);
    let line = format_summary_line(1, "dram-quest", 5, t);
    assert!(line.contains("dram-quest"));
    assert!(line.contains("5"));
    assert!(line.contains("1h ago"));
}

fn dir_result(root: &str, project: &str, mtime: SystemTime) -> DirResult {
    DirResult {
        dir: PathBuf::from(root).join(project),
        files: vec![FileEntry {
            name: ".env".to_string(),
            path: PathBuf::from(root).join(project).join(".env"),
            mtime,
            size: 1,
            type_label: "secrets".to_string(),
        }],
    }
}

#[test]
fn test_summary_groups_sorted_by_last_modified() {
    let root = "/workspace";
    let now = SystemTime::now();
    let results = vec![
        dir_result(root, "older", now - Duration::from_secs(86400 * 3)),
        dir_result(root, "newest", now - Duration::from_secs(3600)),
        dir_result(root, "middle", now - Duration::from_secs(86400)),
    ];

    let groups = build_summary_groups(&results, root);
    let names: Vec<&str> = groups.iter().map(|(name, _, _)| name.as_str()).collect();

    assert_eq!(names, vec!["newest", "middle", "older"]);
}

#[test]
fn test_verbose_groups_sorted_by_project_recency() {
    let root = "/workspace";
    let now = SystemTime::now();
    let results = vec![
        dir_result(root, "older", now - Duration::from_secs(86400 * 3)),
        dir_result(root, "newest", now - Duration::from_secs(3600)),
        dir_result(root, "middle", now - Duration::from_secs(86400)),
    ];

    let groups = build_verbose_groups(&results, root);
    let names: Vec<&str> = groups.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["newest", "middle", "older"]);
}

#[test]
fn test_open_choices_use_project_roots_in_summary_order() {
    let root = "/workspace";
    let now = SystemTime::now();
    let results = vec![
        dir_result(root, "beta", now - Duration::from_secs(7200)),
        dir_result(root, "alpha", now - Duration::from_secs(1800)),
    ];

    let choices = build_open_choices(&results, root);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].project, "alpha");
    assert_eq!(choices[0].path, PathBuf::from("/workspace/alpha"));
    assert_eq!(choices[1].project, "beta");
}

#[test]
fn test_summary_groups_merge_same_project_and_keep_latest_mtime() {
    let root = "/workspace";
    let now = SystemTime::now();
    let results = vec![
        dir_result(root, "alpha", now - Duration::from_secs(86400 * 2)),
        DirResult {
            dir: PathBuf::from(root).join("alpha").join("src"),
            files: vec![FileEntry {
                name: "config.toml".to_string(),
                path: PathBuf::from(root).join("alpha/src/config.toml"),
                mtime: now - Duration::from_secs(1800),
                size: 1,
                type_label: "configs".to_string(),
            }],
        },
        dir_result(root, "beta", now - Duration::from_secs(7200)),
    ];

    let groups = build_summary_groups(&results, root);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "alpha");
    assert_eq!(groups[0].1, 2);
    assert_eq!(groups[1].0, "beta");
}
