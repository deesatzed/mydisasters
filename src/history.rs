use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use chrono::Utc;
use crate::config::config_dir;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub args: Vec<String>,
    pub ran_at: String,
}

impl HistoryEntry {
    pub fn command(&self) -> String {
        format_preset_command(&self.args)
    }
}

impl Serialize for HistoryEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct EntrySer<'a> {
            args: &'a [String],
            ran_at: &'a str,
        }
        EntrySer {
            args: &self.args,
            ran_at: &self.ran_at,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HistoryEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EntryDe {
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            command: String,
            ran_at: String,
        }
        let raw = EntryDe::deserialize(deserializer)?;
        let args = if !raw.args.is_empty() {
            raw.args
        } else {
            raw.command.split_whitespace().map(str::to_string).collect()
        };
        Ok(HistoryEntry { args, ran_at: raw.ran_at })
    }
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
    #[serde(default, deserialize_with = "deserialize_entries")]
    entries: Vec<HistoryEntry>,
    #[serde(default, deserialize_with = "deserialize_presets")]
    presets: HashMap<String, Vec<String>>,
    last_run: Option<LastRun>,
}

fn deserialize_entries<'de, D>(deserializer: D) -> Result<Vec<HistoryEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<HistoryEntry>::deserialize(deserializer)
}

fn deserialize_presets<'de, D>(deserializer: D) -> Result<HashMap<String, Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, serde_json::Value> =
        HashMap::deserialize(deserializer).unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter_map(|(name, value)| match value {
            serde_json::Value::Array(items) => {
                let args: Vec<String> = items
                    .into_iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                (!args.is_empty()).then_some((name, args))
            }
            serde_json::Value::String(cmd) => {
                let args: Vec<String> = cmd.split_whitespace().map(str::to_string).collect();
                (!args.is_empty()).then_some((name, args))
            }
            _ => None,
        })
        .collect())
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
        let path = config_dir().join("dirtrack/history.json");
        Self::new(path)
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.data.entries
    }

    pub fn push_args(&mut self, args: &[String]) {
        self.data.entries.insert(0, HistoryEntry {
            args: args.to_vec(),
            ran_at: Utc::now().to_rfc3339(),
        });
        self.data.entries.truncate(5);
    }

    pub fn get_entry(&self, index: usize) -> Option<&[String]> {
        if index == 0 {
            return None;
        }
        self.data.entries.get(index - 1).map(|e| e.args.as_slice())
    }

    pub fn save_preset(&mut self, name: &str, args: &[String]) {
        self.data.presets.insert(name.to_string(), args.to_vec());
    }

    pub fn get_preset(&self, name: &str) -> Option<&[String]> {
        self.data.presets.get(name).map(|args| args.as_slice())
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

pub fn build_preset_args(
    dir: &str,
    since: &Option<String>,
    type_spec: &Option<String>,
    verbose: bool,
) -> Vec<String> {
    let mut args = vec!["dirtrack".to_string(), dir.to_string()];
    if let Some(s) = since {
        args.push("--since".to_string());
        args.push(s.clone());
    }
    if let Some(t) = type_spec {
        args.push("--type".to_string());
        args.push(t.clone());
    }
    if verbose {
        args.push("--verbose".to_string());
    }
    args
}

pub fn format_preset_command(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{}\"", arg)
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}