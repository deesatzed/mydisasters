use dialoguer::{theme::ColorfulTheme, Input, Select};

pub struct InteractiveResult {
    pub dir: String,
    pub since: Option<String>,
    pub type_spec: Option<String>,
    pub verbose: bool,
    pub open: bool,
}

pub fn run_interactive() -> Result<InteractiveResult, String> {
    let theme = ColorfulTheme::default();

    let default_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let dir: String = Input::with_theme(&theme)
        .with_prompt("Start dir")
        .default(default_dir)
        .interact_text()
        .map_err(|e| e.to_string())?;

    let since_options = &["2h", "7d", "30d", "custom", "no filter"];
    let since_idx = Select::with_theme(&theme)
        .with_prompt("Since when?")
        .items(since_options)
        .default(1)
        .interact()
        .map_err(|e| e.to_string())?;

    let since = match since_idx {
        4 => None,
        3 => {
            let custom: String = Input::with_theme(&theme)
                .with_prompt("Enter value (e.g. 12h, 2026-01-01)")
                .interact_text()
                .map_err(|e| e.to_string())?;
            Some(custom)
        }
        i => Some(since_options[i].to_string()),
    };

    let type_options = &["all", "secrets", "configs", "code", "custom"];
    let type_idx = Select::with_theme(&theme)
        .with_prompt("File types?")
        .items(type_options)
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;

    let type_spec = match type_idx {
        0 => None,
        4 => {
            let custom: String = Input::with_theme(&theme)
                .with_prompt("Enter extensions (e.g. .env,.toml)")
                .interact_text()
                .map_err(|e| e.to_string())?;
            Some(custom)
        }
        i => Some(type_options[i].to_string()),
    };

    let verbose_idx = Select::with_theme(&theme)
        .with_prompt("Show file details?")
        .items(&["summary only", "verbose (show files)"])
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;
    let verbose = verbose_idx == 1;

    let open_idx = Select::with_theme(&theme)
        .with_prompt("Open result in Finder?")
        .items(&["no", "yes (prompt after results)"])
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;
    let open = open_idx == 1;

    Ok(InteractiveResult { dir, since, type_spec, verbose, open })
}
