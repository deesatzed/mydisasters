# dirtrack

> Find directories with recently changed files — across any workspace, instantly.

```
$ dirtrack /Volumes/WS4TB --since 7d --type secrets

/Volumes/WS4TB — since 7d — type: secrets

  #  Project               Changes   Last modified
  ────────────────────────────────────────────────────
  1  dram-quest                5     2h ago
  2  ABXorcist                 2     1d ago
  3  ERSATZ_RAG                1     3d ago

  8 files matched  |  142,847 files scanned  |  0.3s
```

Built for developers with sprawling workspaces who need to answer **"what changed and where?"** without memorizing `find` incantations.

---

## Install

**Requires:** Rust 1.77+ ([install](https://rustup.rs))

```bash
git clone https://github.com/deesatzed/mydisasters.git
cd mydisasters
cargo install --path .
```

Installs `dirtrack` to `~/.cargo/bin/dirtrack` (already on your `$PATH` if you use rustup).

---

## Two ways to use it

### Interactive mode — run bare, no flags needed

```
$ dirtrack

  Start dir:  [/Volumes/WS4TB]
  Since when? > 2h  7d  30d  custom  no filter
  File types? > all  secrets  configs  code  custom
  Show file details? > summary only  verbose
  Open result in file manager? > no  yes

▶ Ran: dirtrack /Volumes/WS4TB --since 7d --type secrets
```

Arrow keys to select. Prompts prefill from your last run. The tool echoes the equivalent command so you learn flags at your own pace.

### Direct mode — flags for speed

```bash
# What secrets files changed this week across my workspace?
dirtrack /Volumes/WS4TB --since 7d --type secrets

# What configs changed in the last 2 hours, with file details?
dirtrack . --since 2h --type configs -v

# Did anyone touch a .env file in the last month?
dirtrack /var/www --since 30d --file .env

# Limit depth to avoid diving into vendored deps
dirtrack . --since 7d --type code --depth 3
```

---

## All flags

```
dirtrack [DIR] [OPTIONS]

ARGS:
  [DIR]             Directory to search (default: current working dir)

OPTIONS:
  --since <value>   Time range start — natural: 2h, 7d, 30m  or  ISO: 2026-01-01
  --until <value>   Time range end (default: now)
  --type <value>    secrets | configs | code | all | custom (.env,.toml)
  --file <name>     Exact filename match — e.g. .env, docker-compose.yml
  --depth <n>       Max recursion depth
  --open            After results, prompt to open a project in your file manager
  -v, --verbose     Show individual files grouped by project, most recent first
  --save <name>     Save current search as a named preset
  --run <target>    Re-run a preset by name, or history entry as !1, !2, ...
  --history         Show last 5 interactive searches
  --completions <shell>  Generate shell completions (bash, zsh, fish, powershell, elvish)
  --refresh         Force a full re-scan, ignoring the cached index
  -h, --help        Print help
  -V, --version     Print version
```

---

## File type presets

| Preset | Extensions matched |
|--------|--------------------|
| `secrets` | `.env` `.key` `.pem` `.p12` `.pfx` `.secret` |
| `configs` | `.yaml` `.yml` `.toml` `.json` `.ini` `.conf` |
| `code` | `.rs` `.ts` `.tsx` `.py` `.go` `.js` |
| `all` | everything (no filter) |
| custom | pass comma-separated: `--type .env,.toml` |

---

## Presets and history

Never retype the same search twice:

```bash
# Save a search (paths with spaces work correctly)
dirtrack "/Volumes/My Disk" --since 1d --type secrets --save daily

# Re-run a named preset
dirtrack --run daily

# See last 5 interactive searches
dirtrack --history

# Re-run the most recent interactive search
dirtrack --run !1
```

Saved at `$XDG_CONFIG_HOME/dirtrack/history.json` (defaults to `~/.config/dirtrack/history.json`). Presets and history store structured args, not shell strings — so paths with spaces survive round-trips.

Direct-mode runs update `last_run` so interactive prompts stay in sync even if you only use flags.

---

## Verbose output

Add `-v` to see every file, not just the directory summary:

```
$ dirtrack /Volumes/WS4TB --since 7d --type secrets -v

dram-quest  (5 changes)
  .env                                 2h ago    secrets
  prisma/.env.local                    3h ago    secrets
  src/lib/.env.test                    6h ago    secrets

ABXorcist  (2 changes)
  config/.env                          1d ago    secrets
  .env.staging                         2d ago    secrets
```

Projects are sorted by most recently modified. Within each project, files are sorted the same way.

---

## Shell completions

```bash
# zsh
dirtrack --completions zsh > "${fpath[1]}/_dirtrack"

# bash
dirtrack --completions bash > /etc/bash_completion.d/dirtrack

# fish
dirtrack --completions fish > ~/.config/fish/completions/dirtrack.fish
```

---

## Automatically skipped

These directories are always excluded to keep results clean:

`target/`  `node_modules/`  `.git/`  `.next/`  `__pycache__/`

---

## Performance

| Workspace size | Time |
|----------------|------|
| ~3,600 files | < 0.1s |
| ~142,000 files | ~0.3s |
| ~14,500,000 files (external HDD) | ~59s |

Rust + `walkdir` — single-threaded, zero heap allocation per file beyond what the OS gives you.

Repeat scans of the same directory skip the tree walk — see "Index cache" below for measured results.

---

## Index cache

Repeat scans of the same directory reuse a cached file list instead of
re-walking the entire tree. The cache lives at `$XDG_CONFIG_HOME/dirtrack/index/`
(defaults to `~/.config/dirtrack/index/`) and is valid for 24 hours.

**On a cache hit:** dirtrack re-checks the modification time of every
previously-seen file via `stat()`, but does **not** discover files created
since the last full scan. Use `--refresh` to force a full walk immediately,
or just wait — the cache auto-expires after 24 hours.

```bash
# Force a fresh full scan right now
dirtrack /Volumes/WS4TB --since 7d --type secrets --refresh
```

**Measured speedup varies by workload:**

| Workspace | Full walk | Cache hit |
|-----------|-----------|-----------|
| ~27 files (local SSD) | 0.19s | 0.006s (~30x) |
| ~6,570,000 files (external HDD) | 137.8s | 110.5s (~1.25x) |

On a small local directory the cache hit is dramatic, since avoiding the directory walk avoids nearly all the work. On a multi-million-file external drive, the cache is still correct (no walk occurs — confirmed by an unchanged cache timestamp) but the win is modest: the bottleneck shifts to issuing millions of individual `stat()` calls, which on a slow/external disk costs nearly as much as walking did. The cache is most valuable for moderate-sized workspaces and SSDs; on very large external-disk trees, expect a smaller improvement.

**Disk usage:** the cache stores one JSON entry per file, so it scales with workspace size — roughly 1.7GB for 6.5 million files. To reclaim space, delete the cache directory: `rm -rf ~/.config/dirtrack/index/`. It will be rebuilt automatically on the next scan.

---

## How it works

Uses the OS `stat()` syscall on every file to read modification time (`mtime`). No daemon, no background process. Results are fresh from the filesystem on every full walk; cache hits re-stat known files only (see "Index cache" above).

**Limitations:**
- A file that was *copied or moved* into a directory retains its original `mtime`, so it may not appear as "new" even though it's new in that location. Files that were *created or edited* will always appear correctly.
- Repeat scans within 24h use a cached file list and won't see brand-new files until the cache refreshes — use `--refresh` to force immediate discovery.

---

## License

MIT
