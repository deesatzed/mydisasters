use rmcp::{
    Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::aggregate::find_across_caches;
use crate::config::config_dir;
use crate::filters::parse_since;
use crate::index;
use crate::scanner::{scan_with_cache, ScanConfig};
use crate::trend::{build_entry, append_entry, deltas_for_root};

fn index_dir() -> PathBuf {
    config_dir().join("dirtrack/index")
}

fn trend_file() -> PathBuf {
    config_dir().join("dirtrack/trend.json")
}

fn system_time_to_rfc3339(t: SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    dt.to_rfc3339()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindChangedRequest {
    /// Directory to scan (absolute or relative path)
    pub dir: String,
    /// Time range start: "2h", "7d", "30m", or ISO date "2026-01-01"
    pub since: Option<String>,
    /// Time range end (defaults to now); same format as `since`
    pub until: Option<String>,
    /// Find directories where every file is older than this point (same format as `since`)
    pub stale: Option<String>,
    /// File type preset (secrets, configs, code, all) or comma-separated extensions like ".env,.toml"
    #[serde(rename = "type")]
    pub type_filter: Option<String>,
    /// Exact filename to match, e.g. ".env"
    pub file: Option<String>,
    /// Maximum directory recursion depth
    pub depth: Option<usize>,
    /// Force a full re-scan, ignoring the cached index
    pub refresh: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileEntryOut {
    pub name: String,
    pub path: String,
    pub mtime: String,
    pub size: u64,
    pub type_label: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DirResultOut {
    pub dir: String,
    pub files: Vec<FileEntryOut>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FindChangedResponse {
    pub results: Vec<DirResultOut>,
    pub total_files_matched: usize,
    pub total_files_scanned: u64,
    pub elapsed_ms: u128,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FuzzyFindRequest {
    /// Case-insensitive substring to search for across all previously scanned directories
    pub pattern: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FuzzyFindMatchOut {
    pub root: String,
    pub path: String,
    pub mtime: String,
    pub size: u64,
    pub cache_age_secs: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FuzzyFindResponse {
    pub matches: Vec<FuzzyFindMatchOut>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrendRequest {
    /// Directory to report growth history for
    pub dir: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TrendDeltaOut {
    pub from_walked_at: String,
    pub to_walked_at: String,
    pub file_count_delta: i64,
    pub total_size_delta: i64,
    pub file_count_after: u64,
    pub total_size_after: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TrendResponse {
    pub deltas: Vec<TrendDeltaOut>,
}

#[derive(Clone)]
pub struct DirtrackServer {
    tool_router: ToolRouter<Self>,
}

impl Default for DirtrackServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl DirtrackServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Scan a directory tree and report which folders contain files matching the given
    /// time/type/name filters. Uses dirtrack's persisted index cache for speed; pass
    /// refresh=true to force a full re-walk.
    #[tool(
        name = "find_changed",
        description = "Scan a directory for files changed/stale within a time window, optionally filtered by file type or exact filename"
    )]
    pub async fn find_changed(
        &self,
        params: Parameters<FindChangedRequest>,
    ) -> Result<Json<FindChangedResponse>, String> {
        let req = params.0;
        let start = std::path::Path::new(&req.dir);

        let since = parse_opt_time(&req.since)?;
        let until = parse_opt_time(&req.until)?;
        let stale_before = parse_opt_time(&req.stale)?;

        let config = ScanConfig {
            since,
            until,
            type_spec: req.type_filter.clone(),
            filename: req.file.clone(),
            max_depth: req.depth,
            stale_before,
        };

        let cache_file = index::cache_path_for_root(&index_dir(), start);
        let refresh = req.refresh.unwrap_or(false);

        let start_time = std::time::Instant::now();
        let (results, scanned) = scan_with_cache(start, &config, &cache_file, refresh)
            .map_err(|e| format!("scan error: {}", e))?;
        let elapsed_ms = start_time.elapsed().as_millis();

        if let Some(cache) = index::read_cache(&cache_file) {
            let entry = build_entry(start, cache.walked_at, &cache.files);
            let _ = append_entry(&trend_file(), entry);
        }

        let total_files_matched: usize = results.iter().map(|r| r.files.len()).sum();

        let results_out = results
            .into_iter()
            .map(|r| DirResultOut {
                dir: r.dir.to_string_lossy().to_string(),
                files: r
                    .files
                    .into_iter()
                    .map(|f| FileEntryOut {
                        name: f.name,
                        path: f.path.to_string_lossy().to_string(),
                        mtime: system_time_to_rfc3339(f.mtime),
                        size: f.size,
                        type_label: f.type_label,
                    })
                    .collect(),
            })
            .collect();

        Ok(Json(FindChangedResponse {
            results: results_out,
            total_files_matched,
            total_files_scanned: scanned,
            elapsed_ms,
        }))
    }

    /// Fuzzy (case-insensitive substring) search across every directory dirtrack has
    /// previously scanned, using cached index data only. Does not touch the filesystem.
    #[tool(
        name = "fuzzy_find",
        description = "Search filenames across all previously scanned directories by substring, using cached data only (no filesystem walk)"
    )]
    pub async fn fuzzy_find(
        &self,
        params: Parameters<FuzzyFindRequest>,
    ) -> Result<Json<FuzzyFindResponse>, String> {
        let req = params.0;
        let matches = find_across_caches(&index_dir(), &req.pattern, SystemTime::now());

        let matches_out = matches
            .into_iter()
            .map(|m| FuzzyFindMatchOut {
                root: m.root.to_string_lossy().to_string(),
                path: m.file.path.to_string_lossy().to_string(),
                mtime: system_time_to_rfc3339(m.file.mtime),
                size: m.file.size,
                cache_age_secs: m.cache_age.as_secs(),
            })
            .collect();

        Ok(Json(FuzzyFindResponse {
            matches: matches_out,
        }))
    }

    /// Report file-count and total-size growth history for a directory, computed from
    /// dirtrack's trend log (populated automatically by prior scans).
    #[tool(
        name = "trend",
        description = "Report file count / total size growth history for a directory, based on past scans"
    )]
    pub async fn trend(
        &self,
        params: Parameters<TrendRequest>,
    ) -> Result<Json<TrendResponse>, String> {
        let req = params.0;
        let dir = Path::new(&req.dir);
        let deltas = deltas_for_root(&trend_file(), dir);

        let deltas_out = deltas
            .into_iter()
            .map(|d| TrendDeltaOut {
                from_walked_at: system_time_to_rfc3339(d.from.walked_at),
                to_walked_at: system_time_to_rfc3339(d.to.walked_at),
                file_count_delta: d.file_count_delta,
                total_size_delta: d.total_size_delta,
                file_count_after: d.to.file_count,
                total_size_after: d.to.total_size,
            })
            .collect();

        Ok(Json(TrendResponse { deltas: deltas_out }))
    }
}

fn parse_opt_time(s: &Option<String>) -> Result<Option<SystemTime>, String> {
    match s {
        Some(s) => parse_since(s).map(Some),
        None => Ok(None),
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DirtrackServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "dirtrack MCP server. Tools: find_changed (scan for changed/stale files by \
                 time window and type), fuzzy_find (cached cross-directory filename search), \
                 trend (file count/size growth history).",
            )
    }
}

pub async fn run_stdio_server() -> anyhow::Result<()> {
    let server = DirtrackServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
