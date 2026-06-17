use std::time::SystemTime;
use colored::Colorize;
use crate::scanner::DirResult;

pub fn format_relative_time(t: SystemTime) -> String {
    let elapsed = t.elapsed().unwrap_or_default();
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub fn format_summary_line(index: usize, project: &str, count: usize, last_mtime: SystemTime) -> String {
    format!(
        "  {}  {:<20}  {:>4} change{}  {}",
        index,
        project,
        count,
        if count == 1 { "" } else { "s" },
        format_relative_time(last_mtime)
    )
}

pub fn print_header(dir: &str, since: &str, type_spec: &str) {
    println!("\n{} — since {} — type: {}",
        dir.cyan().bold(),
        since.yellow(),
        type_spec.yellow()
    );
    println!();
    println!("  {}  {:<20}  {:>7}  {}",
        "#".dimmed(),
        "Project".dimmed(),
        "Changes".dimmed(),
        "Last modified".dimmed()
    );
    println!("  {}", "─".repeat(52).dimmed());
}

pub fn print_summary(results: &[DirResult], start_dir: &str) {
    let start = std::path::Path::new(start_dir);

    // Group by top-level project name, accumulating counts and tracking latest mtime
    let mut groups: Vec<(String, usize, SystemTime)> = Vec::new();

    for result in results {
        let project = result.dir
            .strip_prefix(start)
            .ok()
            .and_then(|p| p.components().next())
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| result.dir.to_string_lossy().to_string());

        let last_mtime = result.files.iter()
            .map(|f| f.mtime)
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if let Some(existing) = groups.iter_mut().find(|(name, _, _)| name == &project) {
            existing.1 += result.files.len();
            if last_mtime > existing.2 {
                existing.2 = last_mtime;
            }
        } else {
            groups.push((project, result.files.len(), last_mtime));
        }
    }

    for (i, (project, count, last_mtime)) in groups.iter().enumerate() {
        println!("{}", format_summary_line(i + 1, project, *count, *last_mtime));
    }
}

pub fn print_verbose(results: &[DirResult], start_dir: &str) {
    let start = std::path::Path::new(start_dir);
    for result in results {
        let rel = result.dir.strip_prefix(start)
            .unwrap_or(&result.dir)
            .to_string_lossy();
        println!("\n{}  ({} change{})",
            rel.cyan().bold(),
            result.files.len(),
            if result.files.len() == 1 { "" } else { "s" }
        );
        for f in &result.files {
            let mtime_str = format_relative_time(f.mtime);
            println!("  {:<35}  {}  {}",
                f.name.white(),
                mtime_str.yellow(),
                f.type_label.dimmed()
            );
        }
    }
}

pub fn print_footer(total_files: usize, scanned: u64, elapsed_ms: u128) {
    println!("\n  {} files matched  |  {} files scanned  |  {:.1}s",
        total_files,
        scanned,
        elapsed_ms as f64 / 1000.0
    );
}

pub fn print_echo_command(cmd: &str) {
    println!("\n{} {}", "▶ Ran:".green().bold(), cmd.white());
}
