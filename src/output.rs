use std::path::{Path, PathBuf};
use std::time::SystemTime;
use colored::Colorize;
use crate::scanner::{DirResult, FileEntry};
use crate::trend::TrendDelta;
use crate::aggregate::FindMatch;
use std::collections::BTreeMap;
use std::time::Duration;

pub fn format_size_delta(bytes: i64) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let abs = bytes.unsigned_abs();
    if abs < 1024 {
        format!("{}{}B", sign, abs)
    } else if abs < 1024 * 1024 {
        format!("{}{:.1}KB", sign, abs as f64 / 1024.0)
    } else if abs < 1024 * 1024 * 1024 {
        format!("{}{:.1}MB", sign, abs as f64 / (1024.0 * 1024.0))
    } else {
        format!("{}{:.1}GB", sign, abs as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn print_trend(dir: &str, deltas: &[TrendDelta]) {
    println!("\n{} — growth trend", dir.cyan().bold());
    if deltas.is_empty() {
        println!("  Not enough scan history yet. Run dirtrack against this directory at least twice.");
        return;
    }
    println!();
    println!("  {:<22}  {:>12}  {:>12}",
        "Scanned at".dimmed(),
        "Files".dimmed(),
        "Size".dimmed()
    );
    for delta in deltas {
        let when = format_relative_time(delta.to.walked_at);
        let files_str = format!("{:+}", delta.file_count_delta);
        let size_str = format_size_delta(delta.total_size_delta);
        println!("  {:<22}  {:>12}  {:>12}", when, files_str, size_str);
    }
}

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

pub fn project_name(result: &DirResult, start: &Path) -> String {
    result.dir
        .strip_prefix(start)
        .ok()
        .and_then(|p| p.components().next())
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| result.dir.to_string_lossy().to_string())
}

pub fn build_summary_groups(results: &[DirResult], start_dir: &str) -> Vec<(String, usize, SystemTime)> {
    let start = Path::new(start_dir);
    let mut groups: Vec<(String, usize, SystemTime)> = Vec::new();

    for result in results {
        let project = project_name(result, start);
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

    groups.sort_by_key(|(_, _, last_mtime)| std::cmp::Reverse(*last_mtime));
    groups
}

pub struct OpenChoice {
    pub project: String,
    pub path: PathBuf,
    pub count: usize,
    pub last_mtime: SystemTime,
}

pub fn build_open_choices(results: &[DirResult], start_dir: &str) -> Vec<OpenChoice> {
    let start = Path::new(start_dir);
    build_summary_groups(results, start_dir)
        .into_iter()
        .map(|(project, count, last_mtime)| OpenChoice {
            path: start.join(&project),
            project,
            count,
            last_mtime,
        })
        .collect()
}

pub fn build_verbose_groups(results: &[DirResult], start_dir: &str) -> Vec<(String, Vec<FileEntry>)> {
    let start = Path::new(start_dir);
    build_summary_groups(results, start_dir)
        .into_iter()
        .map(|(project, _, _)| {
            let mut files: Vec<FileEntry> = results
                .iter()
                .filter(|result| project_name(result, start) == project)
                .flat_map(|result| result.files.iter().cloned())
                .collect();
            files.sort_by_key(|f| std::cmp::Reverse(f.mtime));
            (project, files)
        })
        .collect()
}

pub fn print_summary(results: &[DirResult], start_dir: &str) {
    for (i, (project, count, last_mtime)) in build_summary_groups(results, start_dir).iter().enumerate() {
        println!("{}", format_summary_line(i + 1, project, *count, *last_mtime));
    }
}

pub fn print_verbose(results: &[DirResult], start_dir: &str) {
    let start = Path::new(start_dir);
    for (project, files) in build_verbose_groups(results, start_dir) {
        println!("\n{}  ({} change{})",
            project.cyan().bold(),
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        );
        for f in &files {
            let rel = f.path.strip_prefix(start)
                .unwrap_or(&f.path)
                .to_string_lossy();
            let mtime_str = format_relative_time(f.mtime);
            println!("  {:<35}  {}  {}",
                rel.white(),
                mtime_str.yellow(),
                f.type_label.dimmed()
            );
        }
    }
}

pub fn print_open_prompt(choices: &[OpenChoice]) {
    println!("\nOpen which? [1-{}, or Enter to skip]:", choices.len());
    for (i, choice) in choices.iter().enumerate() {
        println!(
            "  {}  {} ({} change{}, {})",
            i + 1,
            choice.project,
            choice.count,
            if choice.count == 1 { "" } else { "s" },
            format_relative_time(choice.last_mtime)
        );
    }
}

pub fn print_footer(total_files: usize, scanned: u64, elapsed_ms: u128) {
    println!("\n  {} files matched  |  {} files scanned  |  {:.1}s",
        total_files,
        scanned,
        elapsed_ms as f64 / 1000.0
    );
}

pub fn format_cache_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs < 3600 {
        format!("{}m old", secs / 60)
    } else if secs < 86400 {
        format!("{}h old", secs / 3600)
    } else {
        format!("{}d old", secs / 86400)
    }
}

pub fn print_find_results(pattern: &str, matches: &[FindMatch]) {
    println!("\n{} matching \"{}\"", "Search results".cyan().bold(), pattern.yellow());

    if matches.is_empty() {
        println!("  No matches found in any previously scanned directory.");
        return;
    }

    let mut by_root: BTreeMap<PathBuf, Vec<&FindMatch>> = BTreeMap::new();
    for m in matches {
        by_root.entry(m.root.clone()).or_default().push(m);
    }

    for (root, group) in by_root {
        let age = group.iter().map(|m| m.cache_age).min().unwrap_or_default();
        println!("\n  {}  ({})", root.display().to_string().cyan().bold(), format_cache_age(age).dimmed());
        for m in group {
            let rel = m.file.path.strip_prefix(&root).unwrap_or(&m.file.path);
            println!("    {}", rel.display());
        }
    }
}

pub fn print_echo_command(cmd: &str) {
    println!("\n{} {}", "▶ Ran:".green().bold(), cmd.white());
}