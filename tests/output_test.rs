use dirtrack::output::{format_relative_time, format_summary_line};
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
