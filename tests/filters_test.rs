use std::time::{Duration, SystemTime};
use dirtrack::filters::{classify_type, parse_since, matches_type, matches_filename};

#[test]
fn test_parse_since_hours() {
    let cutoff = parse_since("2h").unwrap();
    let expected = SystemTime::now() - Duration::from_secs(2 * 3600);
    let diff = cutoff.duration_since(expected).unwrap_or_default();
    assert!(diff.as_secs() < 5);
}

#[test]
fn test_parse_since_days() {
    let cutoff = parse_since("7d").unwrap();
    let expected = SystemTime::now() - Duration::from_secs(7 * 86400);
    let diff = cutoff.duration_since(expected).unwrap_or_default();
    assert!(diff.as_secs() < 5);
}

#[test]
fn test_parse_since_iso_date() {
    let cutoff = parse_since("2026-01-01").unwrap();
    // 2026-01-01 00:00:00 UTC as unix timestamp = 1767225600
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1767225600);
    assert_eq!(cutoff, expected);
}

#[test]
fn test_parse_since_invalid() {
    assert!(parse_since("badvalue").is_err());
}

#[test]
fn test_matches_type_secrets_preset() {
    assert!(matches_type(".env", "secrets"));
    assert!(matches_type(".pem", "secrets"));
    assert!(!matches_type(".rs", "secrets"));
}

#[test]
fn test_matches_type_configs_preset() {
    assert!(matches_type(".yaml", "configs"));
    assert!(matches_type(".toml", "configs"));
    assert!(!matches_type(".env", "configs"));
}

#[test]
fn test_matches_type_all() {
    assert!(matches_type(".anything", "all"));
    assert!(matches_type(".rs", "all"));
}

#[test]
fn test_matches_type_custom_extensions() {
    assert!(matches_type(".env", ".env,.toml"));
    assert!(matches_type(".toml", ".env,.toml"));
    assert!(!matches_type(".rs", ".env,.toml"));
}

#[test]
fn test_matches_filename_exact() {
    assert!(matches_filename(".env", ".env"));
    assert!(!matches_filename(".env.local", ".env"));
}

#[test]
fn test_classify_type_uses_presets() {
    assert_eq!(classify_type(".env"), "secrets");
    assert_eq!(classify_type(".yaml"), "configs");
    assert_eq!(classify_type(".rs"), "code");
    assert_eq!(classify_type(".md"), "other");
}
