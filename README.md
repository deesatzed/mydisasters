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
  Open result in Finder? > no  yes

▶ Ran: dirtrack /Volumes/WS4TB --since 7d --type secrets
```

Arrow keys to select. The tool echoes the equivalent command so you learn flags at your own pace.

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
  --open            After results, prompt to open a directory in Finder (macOS)
  -v, --verbose     Show individual files under each directory
  --save <name>     Save current search as a named preset
  --run <name>      Re-run a saved preset
  --history         Show last 5 searches
  -h, --help        Print help
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
# Save a search
dirtrack /Volumes/WS4TB --since 1d --type secrets --save daily

# Re-run it any time
dirtrack --run daily

# See last 5 interactive searches
dirtrack --history
```

Saved at `~/.config/dirtrack/history.json` — human-readable JSON.

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

---

## How it works

Uses the OS `stat()` syscall on every file to read modification time (`mtime`). No indexing, no daemon, no background process. Results are always fresh from the filesystem.

**Limitation:** A file that was *copied or moved* into a directory retains its original `mtime`, so it may not appear as "new" even though it's new in that location. Files that were *created or edited* will always appear correctly.

---

## License

MIT
