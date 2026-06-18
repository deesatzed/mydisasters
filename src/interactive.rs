use dialoguer::{theme::ColorfulTheme, Input, Select};
use crate::history::LastRun;

pub struct InteractiveResult {
    pub dir: String,
    pub since: Option<String>,
    pub type_spec: Option<String>,
    pub verbose: bool,
    pub open: bool,
}

pub fn run_interactive(last_run: Option<&LastRun>) -> Result<InteractiveResult, String> {
    let theme = ColorfulTheme::default();

    let default_dir = last_run
        .map(|r| r.dir.clone())
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string());

    let dir: String = Input::with_theme(&theme)
        .with_prompt("Start dir")
        .default(default_dir)
        .interact_text()
        .map_err(|e| e.to_string())?;

    let since_options = &["2h", "7d", "30d", "custom", "no filter"];
    let since_default_idx = match last_run.and_then(|r| r.since.as_deref()) {
        None => 4,
        Some(s) => since_options.iter().position(|&o| o == s).unwrap_or(3),
    };
    let since_idx = Select::with_theme(&theme)
        .with_prompt("Since when?")
        .items(since_options)
        .default(since_default_idx)
        .interact()
        .map_err(|e| e.to_string())?;

    let since = match since_idx {
        4 => None,
        3 => {
            let custom_default = if since_default_idx == 3 {
                last_run.and_then(|r| r.since.clone()).unwrap_or_default()
            } else {
                String::new()
            };
            let custom: String = Input::with_theme(&theme)
                .with_prompt("Enter value (e.g. 12h, 2026-01-01)")
                .default(custom_default)
                .allow_empty(true)
                .interact_text()
                .map_err(|e| e.to_string())?;
            Some(custom)
        }
        i => Some(since_options[i].to_string()),
    };

    let type_options = &["all", "secrets", "configs", "code", "custom"];
    let type_default_idx = match last_run.and_then(|r| r.type_spec.as_deref()) {
        None => 0,
        Some(s) => type_options.iter().position(|&o| o == s).unwrap_or(4),
    };
    let type_idx = Select::with_theme(&theme)
        .with_prompt("File types?")
        .items(type_options)
        .default(type_default_idx)
        .interact()
        .map_err(|e| e.to_string())?;

    let type_spec = match type_idx {
        0 => None,
        4 => {
            let custom_default = if type_default_idx == 4 {
                last_run.and_then(|r| r.type_spec.clone()).unwrap_or_default()
            } else {
                String::new()
            };
            let custom: String = Input::with_theme(&theme)
                .with_prompt("Enter extensions (e.g. .env,.toml)")
                .default(custom_default)
                .allow_empty(true)
                .interact_text()
                .map_err(|e| e.to_string())?;
            Some(custom)
        }
        i => Some(type_options[i].to_string()),
    };

    let verbose_default_idx = if last_run.map(|r| r.verbose).unwrap_or(false) { 1 } else { 0 };
    let verbose_idx = Select::with_theme(&theme)
        .with_prompt("Show file details?")
        .items(&["summary only", "verbose (show files)"])
        .default(verbose_default_idx)
        .interact()
        .map_err(|e| e.to_string())?;
    let verbose = verbose_idx == 1;

    let open_default_idx = if last_run.map(|r| r.open).unwrap_or(false) { 1 } else { 0 };
    let open_idx = Select::with_theme(&theme)
        .with_prompt("Open result in Finder?")
        .items(&["no", "yes (prompt after results)"])
        .default(open_default_idx)
        .interact()
        .map_err(|e| e.to_string())?;
    let open = open_idx == 1;

    Ok(InteractiveResult { dir, since, type_spec, verbose, open })
}
