use tempfile::TempDir;
use dirtrack::history::{History, HistoryEntry, LastRun};

fn history_in_tmp(tmp: &TempDir) -> History {
    History::new(tmp.path().join("history.json"))
}

#[test]
fn test_history_saves_and_loads() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    h.push("dirtrack /tmp --since 7d --type secrets");
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(h2.entries().len(), 1);
    assert_eq!(h2.entries()[0].command, "dirtrack /tmp --since 7d --type secrets");
}

#[test]
fn test_history_capped_at_five() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    for i in 0..7 {
        h.push(&format!("dirtrack /tmp --since {}d", i));
    }
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(h2.entries().len(), 5);
    assert!(h2.entries()[0].command.contains("6d"));
}

#[test]
fn test_preset_save_and_run() {
    let tmp = TempDir::new().unwrap();
    let mut h = history_in_tmp(&tmp);
    h.save_preset("daily", "dirtrack /Volumes/WS4TB --since 1d --type secrets");
    h.save().unwrap();

    let h2 = history_in_tmp(&tmp);
    assert_eq!(
        h2.get_preset("daily").unwrap(),
        "dirtrack /Volumes/WS4TB --since 1d --type secrets"
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
