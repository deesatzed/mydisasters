use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub ran_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastRun {
    pub dir: String,
    pub since: Option<String>,
    pub type_spec: Option<String>,
    pub verbose: bool,
    pub open: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct HistoryFile {
    entries: Vec<HistoryEntry>,
    presets: HashMap<String, String>,
    last_run: Option<LastRun>,
}

pub struct History {
    path: PathBuf,
    data: HistoryFile,
}

impl History {
    pub fn new(path: PathBuf) -> Self {
        let data = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HistoryFile::default()
        };
        Self { path, data }
    }

    pub fn load_default() -> Self {
        let path = home_config_dir().join("dirtrack/history.json");
        Self::new(path)
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.data.entries
    }

    pub fn push(&mut self, command: &str) {
        self.data.entries.insert(0, HistoryEntry {
            command: command.to_string(),
            ran_at: Utc::now().to_rfc3339(),
        });
        self.data.entries.truncate(5);
    }

    pub fn save_preset(&mut self, name: &str, command: &str) {
        self.data.presets.insert(name.to_string(), command.to_string());
    }

    pub fn get_preset(&self, name: &str) -> Option<&str> {
        self.data.presets.get(name).map(|s| s.as_str())
    }

    pub fn last_run(&self) -> Option<&LastRun> {
        self.data.last_run.as_ref()
    }

    pub fn set_last_run(&mut self, run: LastRun) {
        self.data.last_run = Some(run);
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create config dir: {}", e))?;
        }
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| format!("serialize error: {}", e))?;
        std::fs::write(&self.path, json)
            .map_err(|e| format!("write error: {}", e))?;
        Ok(())
    }
}

fn home_config_dir() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config"))
        .unwrap_or_else(|_| PathBuf::from(".config"))
}
