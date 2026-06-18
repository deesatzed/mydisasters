use std::fs;
use tempfile::TempDir;
use dirtrack::history::{History, LastRun, build_preset_args, format_preset_command};

fn history_in_tmp(tmp: &TempDir) -> History {
    History::new(tmp.path().join("history.json"))
}

#[test]
fn test_history_saves_and_loads() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    let args = build_preset_args(
        "/tmp",
        &Some("7d".to_string()),
        &Some("secrets".to_string()),
        false,
    );
    h.push_args(&args);
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(h2.entries().len(), 1);
    assert_eq!(
        h2.entries()[0].command(),
        "dirtrack /tmp --since 7d --type secrets"
    );
}

#[test]
fn test_history_capped_at_five() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    for i in 0..7 {
        let args = build_preset_args(
            "/tmp",
            &Some(format!("{}d", i)),
            &None,
            false,
        );
        h.push_args(&args);
    }
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(h2.entries().len(), 5);
    assert!(h2.entries()[0].command().contains("6d"));
}

#[test]
fn test_history_entry_preserves_path_with_spaces() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    let args = build_preset_args(
        "/Volumes/My Disk/project",
        &Some("7d".to_string()),
        &None,
        false,
    );
    h.push_args(&args);
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(h2.get_entry(1).unwrap()[1], "/Volumes/My Disk/project");
}

#[test]
fn test_legacy_string_history_entry_loads() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("history.json");
    fs::write(
        &path,
        r#"{
  "entries": [
    {
      "command": "dirtrack /tmp --since 7d --type secrets",
      "ran_at": "2026-06-18T12:00:00Z"
    }
  ],
  "presets": {}
}"#,
    )
    .unwrap();

    let h = History::new(path);
    assert_eq!(
        h.get_entry(1).unwrap(),
        &[
            "dirtrack".to_string(),
            "/tmp".to_string(),
            "--since".to_string(),
            "7d".to_string(),
            "--type".to_string(),
            "secrets".to_string(),
        ]
    );
}

#[test]
fn test_preset_save_and_run() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    let args = build_preset_args(
        "/Volumes/WS4TB",
        &Some("1d".to_string()),
        &Some("secrets".to_string()),
        false,
    );
    h.save_preset("daily", &args);
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(
        h2.get_preset("daily").unwrap(),
        &[
            "dirtrack".to_string(),
            "/Volumes/WS4TB".to_string(),
            "--since".to_string(),
            "1d".to_string(),
            "--type".to_string(),
            "secrets".to_string(),
        ]
    );
}

#[test]
fn test_preset_preserves_path_with_spaces() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    let args = build_preset_args(
        "/Volumes/My Disk/project",
        &Some("7d".to_string()),
        &Some("secrets".to_string()),
        false,
    );
    h.save_preset("spaced", &args);
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    let loaded = h2.get_preset("spaced").unwrap();
    assert_eq!(loaded[1], "/Volumes/My Disk/project");
    assert_eq!(
        format_preset_command(loaded),
        "dirtrack \"/Volumes/My Disk/project\" --since 7d --type secrets"
    );
}

#[test]
fn test_preset_serializes_as_arg_array() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    let args = build_preset_args(
        "/Volumes/My Disk/project",
        &Some("1d".to_string()),
        &None,
        false,
    );
    h.save_preset("spaced", &args);
    h.save().unwrap();

    let raw = fs::read_to_string(tmp.path().join("history.json")).unwrap();
    assert!(raw.contains(r#""/Volumes/My Disk/project""#));
    assert!(!raw.contains(r#""dirtrack /Volumes/My Disk/project"#));
}

#[test]
fn test_legacy_string_preset_loads() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("history.json");
    fs::write(
        &path,
        r#"{
  "entries": [],
  "presets": {
    "daily": "dirtrack /Volumes/WS4TB --since 1d --type secrets"
  }
}"#,
    )
    .unwrap();

    let h = History::new(path);
    assert_eq!(
        h.get_preset("daily").unwrap(),
        &[
            "dirtrack".to_string(),
            "/Volumes/WS4TB".to_string(),
            "--since".to_string(),
            "1d".to_string(),
            "--type".to_string(),
            "secrets".to_string(),
        ]
    );
}

#[test]
fn test_missing_preset_returns_none() {
    let tmp = TempDir::new().unwrap();
    let h = history_in_tmp(&tmp);
    assert!(h.get_preset("nonexistent").is_none());
}

#[test]
fn test_last_run_round_trips() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    h.set_last_run(LastRun {
        dir: "/Volumes/WS4TB".to_string(),
        since: Some("7d".to_string()),
        type_spec: Some("secrets".to_string()),
        verbose: true,
        open: false,
    });
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    let last = h2.last_run().expect("last_run should be present");
    assert_eq!(last.dir, "/Volumes/WS4TB");
    assert_eq!(last.since.as_deref(), Some("7d"));
    assert_eq!(last.type_spec.as_deref(), Some("secrets"));
    assert!(last.verbose);
    assert!(!last.open);
}

#[test]
fn test_last_run_defaults_to_none() {
    let tmp = TempDir::new().unwrap();
    let h = history_in_tmp(&tmp);
    assert!(h.last_run().is_none());
}
