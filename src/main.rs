mod cli;
use clap::{CommandFactory, Parser};
use cli::Cli;

use dirtrack::{
    config::config_dir,
    filters::parse_since,
    scanner::ScanConfig,
    output::{print_header, print_summary, print_verbose, print_footer, print_echo_command, build_open_choices, print_open_prompt, print_trend, print_find_results},
    history::{History, LastRun, build_preset_args, format_preset_command},
    interactive::run_interactive,
    trend::{append_entry, build_entry, deltas_for_root},
    aggregate::find_across_caches,
};
use std::time::{Instant, SystemTime};
use std::process::Command;

fn main() {
    let args = Cli::parse();

    if args.mcp {
        let rt = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        if let Err(e) = rt.block_on(dirtrack::mcp::run_stdio_server()) {
            eprintln!("MCP server error: {:?}", e);
            std::process::exit(1);
        }
        return;
    }

    if let Some(shell) = args.completions {
        clap_complete::generate(shell, &mut Cli::command(), "dirtrack", &mut std::io::stdout());
        return;
    }

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
            println!("  !{}  {}  ({})", i + 1, e.command(), e.ran_at);
        }
        println!("\nRe-run with: dirtrack --run !1");
        return;
    }

    // --run <preset>: re-run a saved preset or history entry (!N)
    if let Some(ref run_target) = args.run {
        let history = History::load_default();
        let preset_args = if let Some(num_str) = run_target.strip_prefix('!') {
            num_str
                .parse::<usize>()
                .ok()
                .and_then(|n| history.get_entry(n))
        } else {
            history.get_preset(run_target)
        };

        match preset_args {
            Some(args) if !args.is_empty() => {
                println!(
                    "Running '{}': {}",
                    run_target,
                    format_preset_command(args)
                );
                let _ = Command::new(&args[0]).args(&args[1..]).status();
                return;
            }
            _ => {
                eprintln!(
                    "Target '{}' not found. Use --history to list searches or save a preset with --save.",
                    run_target
                );
                std::process::exit(1);
            }
        }
    }

    // --find: fuzzy search across all previously scanned roots, cache-only (no walk)
    if let Some(ref pattern) = args.find {
        let index_dir = config_dir().join("dirtrack/index");
        let matches = find_across_caches(&index_dir, pattern, SystemTime::now());
        print_find_results(pattern, &matches);
        return;
    }

    // --trend: show growth history for this directory instead of scanning
    if args.trend {
        let dir = args.dir.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });
        let trend_file = config_dir().join("dirtrack/trend.json");
        let deltas = deltas_for_root(&trend_file, std::path::Path::new(&dir));
        print_trend(&dir, &deltas);
        return;
    }

    // Determine interactive vs direct mode
    let is_interactive = args.dir.is_none()
        && args.since.is_none()
        && args.type_filter.is_none()
        && args.file.is_none()
        && args.stale.is_none();

    let history_for_defaults = History::load_default();
    let (dir, since_str, type_spec, verbose, open_finder) = if is_interactive {
        match run_interactive(history_for_defaults.last_run()) {
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

    // Parse stale
    let stale_before: Option<SystemTime> = if let Some(ref s) = args.stale {
        match parse_since(s) {
            Ok(t) => Some(t),
            Err(e) => { eprintln!("Error parsing --stale: {}", e); std::process::exit(1); }
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
        stale_before,
    };

    // Scan with timing (uses index cache; --refresh forces a full walk)
    let cache_dir = config_dir().join("dirtrack/index");
    let cache_file = dirtrack::index::cache_path_for_root(&cache_dir, std::path::Path::new(&dir));

    let start_time = Instant::now();
    let (results, scanned) = match dirtrack::scanner::scan_with_cache(
        std::path::Path::new(&dir), &config, &cache_file, args.refresh,
    ) {
        Ok(r) => r,
        Err(e) => { eprintln!("Scan error: {}", e); std::process::exit(1); }
    };
    let elapsed_ms = start_time.elapsed().as_millis();

    // Append a trend entry from the just-written index cache (no extra filesystem walk).
    if let Some(cache) = dirtrack::index::read_cache(&cache_file) {
        let trend_file = config_dir().join("dirtrack/trend.json");
        let entry = build_entry(std::path::Path::new(&dir), cache.walked_at, &cache.files);
        let _ = append_entry(&trend_file, entry);
    }

    let total_files: usize = results.iter().map(|r| r.files.len()).sum();

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

    let command_args = build_preset_args(&dir, &since_str, &type_spec, verbose);
    let command_display = format_preset_command(&command_args);

    // Echo command in interactive mode and save to history
    if is_interactive {
        print_echo_command(&command_display);
        let mut history = History::load_default();
        history.push_args(&command_args);
        history.set_last_run(LastRun {
            dir: dir.clone(),
            since: since_str.clone(),
            type_spec: type_spec.clone(),
            verbose,
            open: open_finder,
        });
        let _ = history.save();
    } else {
        // Direct mode: update last_run so interactive prompts stay in sync
        let mut history = History::load_default();
        history.set_last_run(LastRun {
            dir: dir.clone(),
            since: since_str.clone(),
            type_spec: type_spec.clone(),
            verbose,
            open: open_finder,
        });
        let _ = history.save();
    }

    // --save preset
    if let Some(ref preset_name) = args.save {
        let mut history = History::load_default();
        history.save_preset(preset_name, &command_args);
        let _ = history.save();
        println!("Preset '{}' saved.", preset_name);
    }

    // --open: prompt to open a project in the system file manager
    if open_finder && !results.is_empty() {
        let choices = build_open_choices(&results, &dir);
        print_open_prompt(&choices);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap_or(0);
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            if let Ok(n) = trimmed.parse::<usize>() {
                if n >= 1 && n <= choices.len() {
                    let path = &choices[n - 1].path;
                    if let Err(e) = opener::open(path) {
                        eprintln!("Could not open {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
}