use tempfile::TempDir;
use dirtrack::history::{History, HistoryEntry};

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
