mod cli;
use clap::Parser;
use cli::Cli;

use dirtrack::{
    filters::parse_since,
    scanner::{scan, ScanConfig},
    output::{print_header, print_summary, print_verbose, print_footer, print_echo_command},
    history::History,
    interactive::run_interactive,
};
use std::time::{Instant, SystemTime};
use std::process::Command;

fn main() {
    let args = Cli::parse();

    // --history: show last 5 searches
    if args.history {
        let history = History::load_default();
        let entries = history.entries();
        if entries.is_empty() {
            println!("No search history yet.");
            return;
        }
        println!("\nLast {} searches:", entries.len());
        for (i, e) in entries.iter().enumerate() {
            println!("  !{}  {}  ({})", i + 1, e.command, e.ran_at);
        }
        return;
    }

    // --run <preset>: re-run a saved preset
    if let Some(ref preset_name) = args.run {
        let history = History::load_default();
        match history.get_preset(preset_name) {
            Some(cmd) => {
                println!("Running preset '{}': {}", preset_name, cmd);
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let _ = Command::new(&parts[0]).args(&parts[1..]).status();
                }
                return;
            }
            None => {
                eprintln!("Preset '{}' not found. Use --history to list saved presets.", preset_name);
                std::process::exit(1);
            }
        }
    }

    // Determine interactive vs direct mode
    let is_interactive = args.dir.is_none()
        && args.since.is_none()
        && args.type_filter.is_none()
        && args.file.is_none();

    let (dir, since_str, type_spec, verbose, open_finder) = if is_interactive {
        match run_interactive() {
            Ok(r) => (r.dir, r.since, r.type_spec, r.verbose, r.open),
            Err(e) => {
                eprintln!("Interactive mode error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let dir = args.dir.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });
        (dir, args.since.clone(), args.type_filter.clone(), args.verbose, args.open)
    };

    // Parse since
    let since: Option<SystemTime> = if let Some(ref s) = since_str {
        match parse_since(s) {
            Ok(t) => Some(t),
            Err(e) => { eprintln!("Error parsing --since: {}", e); std::process::exit(1); }
        }
    } else {
        None
    };

    // Parse until
    let until: Option<SystemTime> = if let Some(ref s) = args.until {
        match parse_since(s) {
            Ok(t) => Some(t),
            Err(e) => { eprintln!("Error parsing --until: {}", e); std::process::exit(1); }
        }
    } else {
        None
    };

    let config = ScanConfig {
        since,
        until,
        type_spec: type_spec.clone(),
        filename: args.file.clone(),
        max_depth: args.depth,
    };

    // Scan with timing
    let start_time = Instant::now();
    let results = scan(std::path::Path::new(&dir), &config);
    let elapsed_ms = start_time.elapsed().as_millis();

    let total_files: usize = results.iter().map(|r| r.files.len()).sum();

    // Count scanned files (second pass for footer stat)
    let scanned = walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u64;

    let since_display = since_str.as_deref().unwrap_or("all time");
    let type_display = type_spec.as_deref().unwrap_or("all");
    print_header(&dir, since_display, type_display);

    if results.is_empty() {
        println!("  No matching directories found.");
    } else if verbose {
        print_verbose(&results, &dir);
    } else {
        print_summary(&results, &dir);
    }

    print_footer(total_files, scanned, elapsed_ms);

    // Echo command in interactive mode and save to history
    if is_interactive {
        let mut cmd = format!("dirtrack {}", dir);
        if let Some(ref s) = since_str { cmd.push_str(&format!(" --since {}", s)); }
        if let Some(ref t) = type_spec { cmd.push_str(&format!(" --type {}", t)); }
        if verbose { cmd.push_str(" --verbose"); }
        print_echo_command(&cmd);

        let mut history = History::load_default();
        history.push(&cmd);
        let _ = history.save();
    }

    // --save preset
    if let Some(ref preset_name) = args.save {
        let mut cmd = format!("dirtrack {}", dir);
        if let Some(ref s) = since_str { cmd.push_str(&format!(" --since {}", s)); }
        if let Some(ref t) = type_spec { cmd.push_str(&format!(" --type {}", t)); }
        let mut history = History::load_default();
        history.save_preset(preset_name, &cmd);
        let _ = history.save();
        println!("Preset '{}' saved.", preset_name);
    }

    // --open: prompt to open a dir in Finder
    if open_finder && !results.is_empty() {
        println!("\nOpen which? [1-{}, or Enter to skip]:", results.len());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap_or(0);
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            if let Ok(n) = trimmed.parse::<usize>() {
                if n >= 1 && n <= results.len() {
                    let path = &results[n - 1].dir;
                    let _ = Command::new("open").arg(path).status();
                }
            }
        }
    }
}
