use std::time::{Duration, SystemTime};
use chrono::NaiveDate;

const SECRETS: &[&str] = &[".env", ".key", ".pem", ".p12", ".pfx", ".secret"];
const CONFIGS: &[&str] = &[".yaml", ".yml", ".toml", ".json", ".ini", ".conf"];
const CODE: &[&str] = &[".rs", ".ts", ".tsx", ".py", ".go", ".js"];

pub fn parse_since(s: &str) -> Result<SystemTime, String> {
    if let Some(n) = s.strip_suffix('h') {
        let hours: u64 = n.parse().map_err(|_| format!("invalid hours: {}", s))?;
        return Ok(SystemTime::now() - Duration::from_secs(hours * 3600));
    }
    if let Some(n) = s.strip_suffix('d') {
        let days: u64 = n.parse().map_err(|_| format!("invalid days: {}", s))?;
        return Ok(SystemTime::now() - Duration::from_secs(days * 86400));
    }
    if let Some(n) = s.strip_suffix('m') {
        let mins: u64 = n.parse().map_err(|_| format!("invalid minutes: {}", s))?;
        return Ok(SystemTime::now() - Duration::from_secs(mins * 60));
    }
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("invalid date/duration '{}'. Use: 2h, 7d, 30m, or 2026-01-01", s))?;
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    let secs = datetime.and_utc().timestamp() as u64;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

pub fn matches_type(file_ext: &str, type_spec: &str) -> bool {
    let ext = file_ext.to_lowercase();
    match type_spec {
        "secrets" => SECRETS.contains(&ext.as_str()),
        "configs" => CONFIGS.contains(&ext.as_str()),
        "code"    => CODE.contains(&ext.as_str()),
        "all"     => true,
        custom    => custom.split(',')
                           .map(|e| e.trim().to_lowercase())
                           .any(|e| e == ext),
    }
}

pub fn matches_filename(filename: &str, filter: &str) -> bool {
    filename == filter
}

pub fn classify_type(ext: &str) -> String {
    let ext_lower = ext.to_lowercase();
    if SECRETS.contains(&ext_lower.as_str()) {
        "secrets".to_string()
    } else if CONFIGS.contains(&ext_lower.as_str()) {
        "configs".to_string()
    } else if CODE.contains(&ext_lower.as_str()) {
        "code".to_string()
    } else {
        "other".to_string()
    }
}
