# dirtrack — Handoff Packet
**Generated:** 2026-06-18
**Branch:** master @ bbe8dd6
**Last Commit:** 2026-06-18 — docs: correct index cache performance claims with measured numbers

---

## Quick Resume Checklist
- [ ] `git clone https://github.com/deesatzed/mydisasters.git && cd mydisasters`
- [ ] `rustup update` (ensure Rust 1.77+; built with 1.96.0)
- [ ] `cargo build` — expect `Finished dev [unoptimized + debuginfo]`
- [ ] `cargo test` — expect `28 passed; 0 failed`
- [ ] `cargo install --path .` — installs `dirtrack` to `~/.cargo/bin/`
- [ ] `dirtrack --help` — verify 12 flags appear, including `--refresh`
- [ ] Review "Known Issues & Tech Debt" section below before making changes

## AI Continuity Checklist
- [ ] This handoff reviewed
- [ ] 5 open debt items imported (see Known Issues section — 2 resolved this session)
- [ ] 0 error references (no runtime errors at close; 1 bug found and fixed during this session's verification — see Recent Changes)
- [ ] Verification suite executed: `cargo test && cargo clippy`
- [ ] Next steps prioritized (P1/P2 — no P0 blockers)

---

## What This Project Does

`dirtrack` is a single Rust binary CLI tool that traverses a local directory tree and reports which subdirectories contain files whose modification time (`mtime`) falls within a specified time window. It is designed for developers with multi-project workspaces who need to quickly answer "what changed recently and where?" without memorizing `find` incantations. As of this session it also caches the file list per scanned root so repeat scans can skip the directory walk.

**Tech Stack:** Rust 1.96 (min 1.77), clap 4, walkdir 2, dialoguer 0.11, colored 2, serde_json 1, chrono 0.4
**Architecture Pattern:** Single-binary CLI — no server, no database, no network I/O

---

## Project Structure

```
dirtrack/                  (repo root IS this directory — no nested folder after clone)
├── src/
│   ├── main.rs          ← Entry point + full control flow (~185 lines)
│   ├── cli.rs           ← clap derive: 12 CLI flags including --refresh (57 lines)
│   ├── scanner.rs       ← filters file list from index cache, ScanConfig, DirResult, scan()/scan_with_cache() (~115 lines)
│   ├── index.rs         ← NEW: per-root index cache, load_or_refresh(), 24h TTL (132 lines)
│   ├── filters.rs       ← parse_since(), matches_type(), matches_filename() (43 lines)
│   ├── output.rs        ← all terminal rendering, grouping logic (112 lines)
│   ├── history.rs       ← JSON persistence, History struct, LastRun (97 lines)
│   ├── interactive.rs   ← dialoguer arrow-key prompt flow, now prefills from LastRun (108 lines)
│   └── lib.rs           ← module re-exports for integration tests (7 lines)
├── tests/
│   ├── filters_test.rs  ← 9 tests: parse_since, matches_type, matches_filename
│   ├── scanner_test.rs  ← 4 tests: recent file, old file, max_depth, filename (unchanged — backward-compat verified)
│   ├── history_test.rs  ← 6 tests: save/load, cap-at-5, preset, missing preset, last_run round-trip, last_run defaults to None
│   ├── index_test.rs    ← NEW: 5 tests: first-scan writes cache, cache-hit re-stats but skips new files, stale cache triggers walk, --refresh forces walk, deleted file dropped
│   └── output_test.rs   ← 4 tests: relative time (min/hr/day), summary line
├── docs/
│   └── index.html       ← GitHub Pages landing page (dark theme, terminal demos)
├── Cargo.toml
├── Cargo.lock
└── README.md
```

**Entry Points:**
- `src/main.rs` — `fn main()` — sole runtime entrypoint

**Key Modules:**

| Module | Path | Purpose | Status |
|--------|------|---------|--------|
| CLI args | `src/cli.rs` | Defines all flags via clap derive | ✅ |
| Scanner | `src/scanner.rs` | Applies filters to the cached/walked file list | ✅ |
| Index cache | `src/index.rs` | Per-root cache, 24h TTL, re-stat fast path | ✅ |
| Filters | `src/filters.rs` | Date/type/name predicates | ✅ |
| Output | `src/output.rs` | Colored terminal rendering | ✅ |
| History | `src/history.rs` | JSON persistence for history, presets, and last_run | ✅ |
| Interactive | `src/interactive.rs` | dialoguer prompt UI, prefills from last run | ✅ (no automated test) |

---

## How to Run

### Local Development

```bash
# One-time setup
git clone https://github.com/deesatzed/mydisasters.git
cd mydisasters
cargo build

# Run (debug)
cargo run -- --help
cargo run -- . --since 1h --type configs

# Install to PATH
cargo install --path .
dirtrack --help
```

**Verify it works:**
```bash
dirtrack /tmp --since 1h
# Expected: header line, summary table or "No matching directories found", footer with scan count

dirtrack /tmp --since 1h --refresh
# Expected: forces a full walk even if a cache exists for /tmp
```

### Tests

```bash
cargo test
```

**Current Status:** 28 passing, 0 failing, 0 skipped (9 filters + 6 history + 5 index + 4 output + 4 scanner)
**Known Failures:** none
**Test warnings:** 2 pre-existing unused-import warnings (`HistoryEntry` in `history_test.rs`, similarly cosmetic pattern elsewhere) — does not affect correctness

### Verification Suite

```bash
cargo test && cargo clippy 2>&1 | grep -c "^warning:"
```

**Pass Condition:** `cargo test` exits 0 with `28 passed; 0 failed` across all 5 test files. Clippy emits exactly 2 known pre-existing warnings (`sort_by_key` suggestion, borrowed-expression suggestion) — both pre-date this session, neither blocking.

---

## Current State Assessment

### What's Working ✅

- **Direct mode** — `dirtrack /path --since 7d --type secrets` — verified against the real 14.6M-file `/Volumes/WS4TB` workspace
- **Interactive mode** — bare `dirtrack` triggers arrow-key prompts, echoes command, saves to history
- **Interactive defaults from last run** — NEW: prompts now prefill dir/since/type/verbose/open from the previous interactive run instead of static defaults; falls back to static defaults on first-ever run (no history file)
- **Index cache** — NEW: repeat scans of the same root skip the directory walk and re-stat only previously-seen files; cache auto-expires after 24h; `--refresh` forces an immediate full walk. Verified correct via `walked_at` timestamp staying unchanged on a true cache hit.
- **All 4 filter types** — `secrets`, `configs`, `code`, `all`, custom extensions — 9 tests pass
- **Scanner** — mtime/type/depth/filename filtering, auto-skips `target/`, `.git/`, `node_modules/`, `.next/`, `__pycache__/` — 4 tests pass, backward-compatible `scan()` wrapper preserved for existing callers
- **Output grouping** — results grouped by top-level project name, not per-subdirectory — verified
- **History persistence** — push (capped at 5), save/load JSON, named presets, last_run — 6 tests pass
- **Output formatting** — colored tables, relative timestamps (Xs/Xm/Xh/Xd ago) — 4 tests pass
- **`--save` / `--run`** — preset save and re-run verified manually
- **`--history`** — displays last 5 interactive runs
- **`--open`** — Finder integration verified on macOS
- **`--refresh`** — NEW flag, verified to force full walk and discover files created since the last cached scan
- **Install** — `cargo install --path .` → `~/.cargo/bin/dirtrack` verified
- **GitHub Pages** — `docs/index.html` pushed; enable at repo Settings → Pages → `master /docs`
- **README** — full usage docs with ASCII demo, flag table, preset table, measured (not estimated) performance numbers for both small and very large workspaces
- **Clone instructions** — FIXED this session: README and landing page previously said `cd mydisasters/dirtrack`, which is wrong since the repo root already is the project; corrected to `cd mydisasters`

### What's Incomplete ⚠️

- **`--history` in direct mode** — direct-mode runs are still not recorded; only interactive runs save to history and last_run. Unchanged from prior session — still P2, not addressed this session (was scoped out of this round of work).
- **Interactive mode has no automated test** — wraps terminal I/O; cannot be driven headlessly without PTY harness. Manually verified only (confirmed dialoguer fails with "not a terminal" under piped/non-TTY execution, as expected).
- **GitHub Pages not yet enabled** — `docs/index.html` is pushed but requires one manual step in repo settings to activate.
- **Index cache disk usage** — cache size scales with file count (~1.7GB for 6.5M files in real-world test). No size cap, no automatic pruning. Documented in README; user explicitly chose to ship as-is rather than add compaction or limits.
- **Index cache speedup is workload-dependent** — dramatic on small/local directories (~30x measured), but modest on very large external-disk trees (~1.25x measured against 6.57M files) because the bottleneck shifts to per-file `stat()` syscalls rather than tree walking. This is an inherent trade-off of the chosen "always trust cache, re-stat known files" design — not a bug.

### What's Broken ❌

- Nothing currently broken.

### Current Blockers 🚧

- None. All P1/P2 items are improvements, not blockers.

### Feature Completion Matrix

| Feature | Status | Evidence | Gap to Done | Priority |
|---------|--------|----------|-------------|----------|
| Direct mode scanning | ✅ | `src/scanner.rs`, 4 tests | — | — |
| Interactive mode | ✅ | `src/interactive.rs` | No automated test | P2 |
| Interactive last-run defaults | ✅ | `src/interactive.rs`, `src/history.rs` `LastRun`, 2 tests | — | — |
| Index cache | ✅ | `src/index.rs`, 5 tests, real-world verified | Disk usage unbounded; speedup workload-dependent | P2 (documented trade-offs, not bugs) |
| `--refresh` flag | ✅ | `src/cli.rs`, `src/main.rs`, manually verified | — | — |
| Date filtering (--since/--until) | ✅ | `src/filters.rs`, 4 tests | `--until` semantic (see debt) | P2 |
| Type presets | ✅ | `src/filters.rs`, 5 tests | — | — |
| Custom extensions | ✅ | `filters_test.rs:test_matches_type_custom_extensions` | — | — |
| History persistence | ✅ | `src/history.rs`, 6 tests | Direct mode not saved | P2 |
| Named presets | ✅ | `src/history.rs`, manual test | Space-in-path bug | P1 |
| Output grouping | ✅ | `src/output.rs:46` | — | — |
| Verbose mode | ✅ | `src/output.rs:65`, manual test | — | — |
| Finder open (--open) | ✅ | `src/main.rs`, macOS only | Non-macOS silent fail | P2 |
| Landing page | ✅ | `docs/index.html` pushed | GitHub Pages not enabled | P2 |
| README clone instructions | ✅ | Fixed this session | — | — |
| Test coverage | ✅ | 28/28 pass | interactive.rs untested (documented limitation) | P2 |

---

## Recent Changes

| Date | SHA | Change | Why |
|------|-----|--------|-----|
| 2026-06-18 | bbe8dd6 | Correct index cache performance claims with measured numbers | Real-world test against 6.57M files showed only ~1.25x speedup, not the assumed dramatic win — corrected README to avoid overclaiming |
| 2026-06-18 | 85d3599 | Document index cache behavior, TTL, and --refresh flag | README completeness |
| 2026-06-18 | 35e12f6 | Wire index cache into scanner, add --refresh flag, eliminate double walkdir pass | Closes a P1 tech debt item for free while integrating the cache; verified 0.186s→0.006s on a small dir |
| 2026-06-18 | dba4341 | Add per-root index cache with 24h TTL re-stat fast path | Core caching feature; **found and fixed a real bug during verification**: cache lookup compared raw vs. canonicalized root path, so on macOS (`/var` → `/private/var` symlink) the cache was never hit — both sides now canonicalize before comparing |
| 2026-06-18 | 7a9f5e7 | Prefill interactive mode defaults from last run | User feedback after independent real-world use: static defaults were annoying on repeat runs |
| 2026-06-18 | 1ebe12e | Add last_run field to history for interactive default prefill | Data-layer groundwork for the above |
| 2026-06-18 | c677ed6 | Fix incorrect clone path (repo root is the project, no nested dirtrack/ folder) | User found this trying to follow the README from a fresh clone |
| 2026-06-18 | 4973fbb | Add engineering handoff packet for dirtrack v0.1.0 | Session continuity artifact (this file's predecessor) |
| 2026-06-17 | d915d0d | Add GitHub Pages landing page + upgrade README | UX/docs — "wow" landing page requested |
| 2026-06-17 | f7ee6d0 | Initial README | First docs pass |

**Uncommitted Changes:** none
**Stashed Work:** none

**Notable bug found and fixed this session:** the index cache's root-path comparison initially compared a canonicalized stored path against a non-canonicalized incoming path. On macOS this meant the cache was silently never hit (every scan fell through to a full walk) because `/tmp` resolves to `/private/tmp` and similar for `/var`. Caught during Task 4's verification step (a test failure that reproduced 100% of the time, not a flake) by reasoning through plausible causes before patching — root cause confirmed via a standalone `rustc` snippet proving the symlink resolution, then fixed by canonicalizing both sides of the comparison.

---

## Configuration & Secrets

### Environment Variables

None required. dirtrack reads no environment variables at runtime except `HOME` (used to resolve `~/.config/dirtrack/history.json` and `~/.config/dirtrack/index/`).

| Variable | Purpose | Where |
|----------|---------|-------|
| `HOME` | Resolve config dir | Set by OS automatically |

### External Dependencies

None. Zero network I/O. No databases, no APIs, no auth.

| Service | Purpose | Local Alternative |
|---------|---------|-------------------|
| Filesystem (`stat` syscall) | Read file mtimes | Always available |
| macOS `open` command | Open dirs in Finder | Skip `--open` on non-macOS |

---

## Known Issues & Tech Debt

- [ ] **P1 — `--run` breaks on paths with spaces** — Preset commands stored as single strings and split on whitespace at execution time (`main.rs` `--run` handler). A saved preset containing `/Volumes/My Disk/project` will fail silently. Fix: store presets as `Vec<String>` args in JSON, or use shell-quote parsing. **Unresolved — not in scope this session.**

- [x] **P1 — Double walkdir pass for footer count** — RESOLVED this session. `scan_with_cache()` now returns `(Vec<DirResult>, u64)` with the count sourced directly from the index cache layer; the second `walkdir` pass in `main.rs` was deleted entirely.

- [ ] **P2 — `classify_type` duplicates constants from `filters.rs`** — `scanner.rs` re-implements the SECRETS/CONFIGS/CODE mapping that already exists in `filters.rs`. If a new type is added to `filters.rs`, the type label in verbose output won't update. Fix: move `classify_type` into `filters.rs` and import it. **Unresolved.**

- [ ] **P2 — `--until` semantic ambiguity** — `parse_since()` is reused for `--until`. Passing `2h` to `--until` returns "2 hours ago", not "2 hours from now" — wrong semantics for an end bound. **Unresolved.**

- [ ] **P2 — Direct-mode runs not saved to history** — History save (including the new `last_run`) is still guarded by `if is_interactive`. Users who only use flags will always see "No search history yet" and never get prefilled defaults either, since they never run interactive mode. **Unresolved — deliberately out of scope this session** (this session's last-run feature only benefits interactive-mode users, by design).

- [ ] **P2 — Unused import warnings** — `tests/history_test.rs` imports `HistoryEntry` but never uses it. Cosmetic, pre-existing. **Unresolved.**

- [ ] **P2 — No `--version` flag** — Fix: add `#[command(version)]` to clap derive in `src/cli.rs`. **Unresolved.**

- [ ] **P2 — `--open` macOS-only, silent on Linux** — `main.rs` calls `Command::new("open")`. Fix: gate with `#[cfg(target_os = "macos")]` or use `opener` crate for cross-platform support. **Unresolved.**

- [ ] **P2 — NEW: Index cache has no size cap or pruning** — Cache files scale with file count (~1.7GB observed for 6.5M files). No automatic cleanup; user must `rm -rf ~/.config/dirtrack/index/` manually. Explicitly deferred — user chose "ship as-is, document the cost" over adding compaction/limits when asked directly.

- [ ] **P2 — NEW: Index cache speedup is disk-dependent** — On slow/external/spinning disks with very large file counts, re-stat-per-file can approach the cost of a full walk (measured ~1.25x speedup vs. the ~30x seen on small/SSD workloads). Not a bug — inherent to the "always trust cache, re-stat on hit" design the user explicitly chose over walk-but-skip-unchanged-subdirs or TTL-only alternatives.

---

## Next Steps (Priority Order)

1. **Fix `--run` path-with-spaces bug** — Change `HistoryFile.presets` value from `String` to `Vec<String>` in `src/history.rs`. Update `save_preset()` to store split args, update `--run` handler to use `Command::new(&args[0]).args(&args[1..])`. Update `history_test.rs` for new schema. This is the only remaining correctness bug (P1).

2. **Enable GitHub Pages** — Repo Settings → Pages → Source: `master` branch, `/docs` folder. Landing page is already built and pushed at `docs/index.html`. One click.

3. **Deduplicate `classify_type`** — Move `fn classify_type()` from `src/scanner.rs` into `src/filters.rs` alongside the existing constants. Import it in scanner. Eliminates divergence risk.

4. **Decide on direct-mode history/last-run** — Either accept the current interactive-only scope permanently, or extend `last_run`/`history.push()` saves to direct-mode invocations too (would need a decision on whether direct-mode flag combos should silently start influencing future interactive defaults — not obviously desired, needs a product decision, not just a code change).

5. **Remove unused import warnings** — `tests/history_test.rs` `HistoryEntry`. One-liner cleanup.

6. **Add `--version` flag** — `#[command(version)]` in `src/cli.rs`.

---

## Key Files Reference

| File | Purpose | When to Modify |
|------|---------|----------------|
| `src/main.rs` | Control flow, mode dispatch, all flag handling | Adding new top-level behavior or flags |
| `src/cli.rs` | All CLI argument definitions | Adding/removing/renaming flags |
| `src/scanner.rs` | Filters the cached/walked file list into `DirResult`s | Changing filter logic, output structure |
| `src/index.rs` | Per-root index cache, TTL, re-stat fast path | Changing cache strategy, TTL duration, or what triggers a full walk |
| `src/filters.rs` | Date/type/name predicate logic | Adding new type presets or time formats |
| `src/output.rs` | All terminal rendering | Changing display format or grouping |
| `src/history.rs` | JSON persistence schema (entries, presets, last_run) | Changing what gets saved or history format |
| `src/interactive.rs` | Arrow-key prompt flow, last-run prefill | Adding/reordering interactive prompts, changing default logic |
| `~/.config/dirtrack/history.json` | Runtime state | Never edit manually; deleted to reset history and last-run defaults |
| `~/.config/dirtrack/index/*.json` | Per-root cache files | Never edit manually; delete to force a full re-walk on next scan of that root |
| `docs/index.html` | GitHub Pages landing page | Marketing/UX updates |

---

## Open Questions / Decisions Needed

- **Should direct-mode runs be saved to history and last_run?** Currently only interactive runs are recorded. Carried over unresolved from the prior handoff — no decision made this session.

- **Cross-platform support?** `--open` is macOS-only. If Linux support matters, swap `Command::new("open")` for the `opener` crate. Low effort. Carried over.

- **History cap of 5** — still hardcoded. Carried over, not blocking.

- **Index cache TTL of 24h** — chosen by the user as the balance between safety and control (auto-refresh + manual `--refresh` escape hatch). Revisit only if 24h proves too long/short in practice; no evidence yet either way.

- **Index cache size cap** — explicitly NOT added this session per user decision ("ship as-is, document the cost"). Revisit if disk usage becomes a real complaint, not preemptively.

- **GitHub Pages URL** — once enabled, will be `https://deesatzed.github.io/mydisasters/`. No changes needed post-activation. Carried over.

---

## Appendix: Machine-Readable Summary

```json
{
  "project": "dirtrack",
  "generated": "2026-06-18",
  "repo": {
    "branch": "master",
    "commit": "bbe8dd6",
    "commit_date": "2026-06-18",
    "uncommitted_changes": false,
    "stashed_work": 0
  },
  "stack": {
    "language": "Rust",
    "language_version": "1.96.0",
    "framework": "clap + walkdir + dialoguer",
    "framework_version": "clap 4, walkdir 2, dialoguer 0.11"
  },
  "health": {
    "tests_passing": 28,
    "tests_failing": 0,
    "tests_skipped": 0,
    "lint_clean": false,
    "lint_warnings": 2,
    "lint_notes": "2 pre-existing clippy style warnings (sort_by_key, borrowed ref) — both pre-date this session, no new warnings introduced"
  },
  "status": {
    "working": [
      "direct mode scanning",
      "interactive mode",
      "interactive last-run defaults (new)",
      "index cache with 24h TTL and re-stat fast path (new)",
      "--refresh flag (new)",
      "date filtering (--since/--until)",
      "type presets (secrets/configs/code/all/custom)",
      "history persistence including last_run",
      "named presets (--save/--run)",
      "output grouping by project",
      "verbose mode",
      "Finder open (--open, macOS)",
      "cargo install",
      "README with corrected clone path and measured (not estimated) cache performance",
      "GitHub Pages landing page (pushed, not yet activated)"
    ],
    "incomplete": [
      "interactive mode has no automated test",
      "GitHub Pages not yet enabled in repo settings",
      "direct-mode runs not saved to history or last_run",
      "index cache has no size cap or pruning mechanism"
    ],
    "broken": [],
    "blockers": []
  },
  "continuity": {
    "previous_handoff_loaded": true,
    "assumptions_imported": 6,
    "debt_items_imported": 6,
    "error_refs_imported": 1
  },
  "feature_completion_matrix": [
    {"feature": "direct mode scanning", "status": "✅", "evidence": "src/scanner.rs", "priority": "done"},
    {"feature": "interactive mode", "status": "✅", "evidence": "src/interactive.rs", "priority": "P2 (no automated test)"},
    {"feature": "interactive last-run defaults", "status": "✅", "evidence": "src/interactive.rs, src/history.rs LastRun", "priority": "done"},
    {"feature": "index cache", "status": "✅", "evidence": "src/index.rs, 5 tests, real-world verified", "priority": "P2 (disk usage, workload-dependent speedup — both documented trade-offs)"},
    {"feature": "--refresh flag", "status": "✅", "evidence": "src/cli.rs, src/main.rs", "priority": "done"},
    {"feature": "date filtering", "status": "✅", "evidence": "src/filters.rs", "priority": "P2 (--until semantic)"},
    {"feature": "type presets", "status": "✅", "evidence": "src/filters.rs", "priority": "done"},
    {"feature": "history persistence", "status": "✅", "evidence": "src/history.rs", "priority": "P2 (direct mode not saved)"},
    {"feature": "named presets", "status": "✅", "evidence": "src/history.rs", "priority": "P1 (space-in-path bug)"},
    {"feature": "output grouping", "status": "✅", "evidence": "src/output.rs", "priority": "done"},
    {"feature": "README clone instructions", "status": "✅", "evidence": "README.md, docs/index.html", "priority": "done (fixed this session)"},
    {"feature": "GitHub Pages", "status": "⚠️", "evidence": "docs/index.html", "priority": "P2 (one-click activation)"}
  ],
  "verification_suite": {
    "command": "cargo test && cargo clippy 2>&1 | grep -c '^warning:'",
    "pass_condition": "cargo test exits 0, 28 passed 0 failed across 5 test files; clippy emits exactly 2 known pre-existing warnings",
    "result": "pass"
  },
  "next_steps": [
    {"task": "Fix --run preset execution for paths with spaces", "priority": "P1", "scope": "small"},
    {"task": "Enable GitHub Pages in repo settings", "priority": "P2", "scope": "small"},
    {"task": "Deduplicate classify_type into filters.rs", "priority": "P2", "scope": "small"},
    {"task": "Decide on direct-mode history/last-run scope", "priority": "P2", "scope": "small — needs a product decision, not just code"},
    {"task": "Remove unused HistoryEntry import warning", "priority": "P2", "scope": "small"},
    {"task": "Add --version flag", "priority": "P2", "scope": "small"}
  ]
}
```
