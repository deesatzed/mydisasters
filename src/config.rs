use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config"))
        .unwrap_or_else(|_| PathBuf::from(".config"))
}