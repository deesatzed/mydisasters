use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "dirtrack",
    about = "Find directories with recently changed files",
    long_about = "Scans a directory tree and reports which folders contain files \
                  changed within a time window. Run with no args for interactive mode."
)]
pub struct Cli {
    /// Directory to search (defaults to current working dir)
    pub dir: Option<String>,

    /// Time range start: 2h, 7d, 30m, or ISO date 2026-01-01
    #[arg(long)]
    pub since: Option<String>,

    /// Time range end (defaults to now)
    #[arg(long)]
    pub until: Option<String>,

    /// File type preset or comma-separated extensions (secrets, configs, code, all, or .env,.toml)
    #[arg(long = "type")]
    pub type_filter: Option<String>,

    /// Specific filename to search for (exact match, e.g. .env)
    #[arg(long)]
    pub file: Option<String>,

    /// Maximum directory recursion depth
    #[arg(long)]
    pub depth: Option<usize>,

    /// After results, prompt to open a directory in Finder
    #[arg(long)]
    pub open: bool,

    /// Show individual files under each directory
    #[arg(long, short)]
    pub verbose: bool,

    /// Save current flags as a named preset
    #[arg(long)]
    pub save: Option<String>,

    /// Re-run a saved preset by name
    #[arg(long)]
    pub run: Option<String>,

    /// Show last 5 searches and re-run by number
    #[arg(long)]
    pub history: bool,
}
