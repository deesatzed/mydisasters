use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};

use dirtrack::mcp::{DirtrackServer, FindChangedRequest, FuzzyFindRequest, TrendRequest};
use rmcp::handler::server::wrapper::Parameters;

/// Env vars are process-wide and cargo runs tests concurrently, so every test that
/// touches XDG_CONFIG_HOME must hold this lock for its full duration.
static ENV_LOCK: Mutex<()> = Mutex::const_new(());

/// Points XDG_CONFIG_HOME at a fresh temp dir so MCP tool calls never touch the
/// user's real ~/.config/dirtrack cache. Restores the previous value on drop.
/// Mirrors the override pattern in config_test.rs, extended with the lock above.
struct IsolatedConfig {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    prev: Option<String>,
}

impl IsolatedConfig {
    async fn new() -> Self {
        let guard = ENV_LOCK.lock().await;
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("xdg-config");
        fs::create_dir_all(&custom).unwrap();

        let prev = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", &custom);

        Self {
            _guard: guard,
            _tmp: tmp,
            prev,
        }
    }

    fn path(&self) -> &std::path::Path {
        self._tmp.path()
    }
}

impl Drop for IsolatedConfig {
    fn drop(&mut self) {
        if let Some(value) = &self.prev {
            std::env::set_var("XDG_CONFIG_HOME", value);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }
}

#[tokio::test]
async fn test_find_changed_returns_matching_files() {
    let cfg = IsolatedConfig::new().await;
    let project = cfg.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".env"), b"SECRET=1").unwrap();

    let server = DirtrackServer::new();
    let req = FindChangedRequest {
        dir: project.to_string_lossy().to_string(),
        since: None,
        until: None,
        stale: None,
        type_filter: Some("secrets".to_string()),
        file: None,
        depth: None,
        refresh: Some(true),
    };

    let result = server.find_changed(Parameters(req)).await.unwrap();
    let resp = result.0;

    assert_eq!(resp.total_files_matched, 1);
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].files[0].name, ".env");
    assert_eq!(resp.results[0].files[0].type_label, "secrets");
}

#[tokio::test]
async fn test_find_changed_since_excludes_old_file() {
    let cfg = IsolatedConfig::new().await;
    fs::write(cfg.path().join("old.env"), b"OLD=1").unwrap();

    // since=now means old.env's already-written mtime should be excluded.
    std::thread::sleep(Duration::from_millis(10));

    let server = DirtrackServer::new();
    let req = FindChangedRequest {
        dir: cfg.path().to_string_lossy().to_string(),
        since: Some("0m".to_string()),
        until: None,
        stale: None,
        type_filter: Some("secrets".to_string()),
        file: None,
        depth: None,
        refresh: Some(true),
    };

    let result = server.find_changed(Parameters(req)).await.unwrap();
    assert_eq!(result.0.total_files_matched, 0);
}

#[tokio::test]
async fn test_fuzzy_find_matches_cached_files_by_substring() {
    let cfg = IsolatedConfig::new().await;
    let project = cfg.path().join("dram-quest");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".env.production"), b"X=1").unwrap();

    let server = DirtrackServer::new();

    let scan_req = FindChangedRequest {
        dir: project.to_string_lossy().to_string(),
        since: None,
        until: None,
        stale: None,
        type_filter: Some("all".to_string()),
        file: None,
        depth: None,
        refresh: Some(true),
    };
    server.find_changed(Parameters(scan_req)).await.unwrap();

    let find_req = FuzzyFindRequest {
        pattern: "production".to_string(),
    };
    let result = server.fuzzy_find(Parameters(find_req)).await.unwrap();

    assert_eq!(result.0.matches.len(), 1);
    assert!(result.0.matches[0].path.ends_with(".env.production"));
}

#[tokio::test]
async fn test_fuzzy_find_no_match_returns_empty() {
    let _cfg = IsolatedConfig::new().await;
    let server = DirtrackServer::new();
    let req = FuzzyFindRequest {
        pattern: "this-pattern-should-not-exist-anywhere".to_string(),
    };
    let result = server.fuzzy_find(Parameters(req)).await.unwrap();
    assert!(result.0.matches.is_empty());
}

#[tokio::test]
async fn test_trend_reports_growth_between_scans() {
    let cfg = IsolatedConfig::new().await;
    let project = cfg.path().join("growing");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("a.txt"), b"one").unwrap();

    let server = DirtrackServer::new();
    let dir_str = project.to_string_lossy().to_string();

    let req1 = FindChangedRequest {
        dir: dir_str.clone(),
        since: None,
        until: None,
        stale: None,
        type_filter: Some("all".to_string()),
        file: None,
        depth: None,
        refresh: Some(true),
    };
    server.find_changed(Parameters(req1)).await.unwrap();

    fs::write(project.join("b.txt"), b"two").unwrap();
    let req2 = FindChangedRequest {
        dir: dir_str.clone(),
        since: None,
        until: None,
        stale: None,
        type_filter: Some("all".to_string()),
        file: None,
        depth: None,
        refresh: Some(true),
    };
    server.find_changed(Parameters(req2)).await.unwrap();

    let trend_req = TrendRequest { dir: dir_str };
    let result = server.trend(Parameters(trend_req)).await.unwrap();

    assert_eq!(result.0.deltas.len(), 1, "two scans should produce one delta");
    assert_eq!(result.0.deltas[0].file_count_delta, 1);
    assert_eq!(result.0.deltas[0].file_count_after, 2);
}

#[tokio::test]
async fn test_trend_with_fewer_than_two_scans_is_empty() {
    let cfg = IsolatedConfig::new().await;
    let server = DirtrackServer::new();
    let req = TrendRequest {
        dir: cfg.path().to_string_lossy().to_string(),
    };
    let result = server.trend(Parameters(req)).await.unwrap();
    assert!(result.0.deltas.is_empty());
}

#[tokio::test]
async fn test_find_changed_invalid_since_returns_error() {
    let cfg = IsolatedConfig::new().await;
    let server = DirtrackServer::new();
    let req = FindChangedRequest {
        dir: cfg.path().to_string_lossy().to_string(),
        since: Some("not-a-valid-duration".to_string()),
        until: None,
        stale: None,
        type_filter: None,
        file: None,
        depth: None,
        refresh: Some(true),
    };

    let result = server.find_changed(Parameters(req)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_changed_exact_filename_filter() {
    let cfg = IsolatedConfig::new().await;
    fs::write(cfg.path().join(".env"), b"A=1").unwrap();
    fs::write(cfg.path().join(".env.local"), b"B=2").unwrap();

    let server = DirtrackServer::new();
    let req = FindChangedRequest {
        dir: cfg.path().to_string_lossy().to_string(),
        since: None,
        until: None,
        stale: None,
        type_filter: None,
        file: Some(".env".to_string()),
        depth: None,
        refresh: Some(true),
    };

    let result = server.find_changed(Parameters(req)).await.unwrap();
    assert_eq!(result.0.total_files_matched, 1);
    assert_eq!(result.0.results[0].files[0].name, ".env");
}
