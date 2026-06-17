# dirtrack

A fast Rust CLI that finds directories containing recently changed or added files across a project workspace.

Built for messy multi-project setups where you need to answer "what changed recently and where?" without memorizing `find` flags.

## Install

```bash
git clone https://github.com/deesatzed/mydisasters.git
cd mydisasters/dirtrack
cargo install --path .
```

Requires Rust 1.77+. Binary installs to `~/.cargo/bin/dirtrack`.

## Usage

### Interactive mode (no args)

Run bare and arrow-key through prompts. The tool echoes the equivalent command so you learn flags naturally:

```
$ dirtrack

  Start dir:  [/Volumes/WS4TB]
  Since when? > 2h  7d  30d  custom
  File types? > all  secrets  configs  code  custom
  Show file details? > summary only  verbose
  Open result in Finder? > no  yes

▶ Ran: dirtrack /Volumes/WS4TB --since 7d --type secrets
```

### Direct mode

```bash
dirtrack /Volumes/WS4TB --since 7d --type secrets
dirtrack . --since 2h --type configs -v
dirtrack /path/to/project --file .env --since 30d
```

### Output

**Summary (default):**
```
/Volumes/WS4TB — since 7d — type: secrets

  #  Project               Changes   Last modified
  ────────────────────────────────────────────────────
  1  dram-quest                5     2h ago
  2  ABXorcist                 2     1d ago
  3  ERSATZ_RAG                1     3d ago

  8 files matched  |  142847 files scanned  |  0.3s
```

**Verbose (`-v`):**
```
dram-quest  (5 changes)
  .env                                 2h ago   secrets
  prisma/.env.local                    3h ago   secrets
```

## All Flags

```
dirtrack [DIR]

  DIR                     Directory to search (default: current working dir)
  --since <value>         2h, 7d, 30m, or ISO date 2026-01-01
  --until <value>         End of date range (default: now)
  --type <value>          secrets | configs | code | all | .env,.toml
  --file <name>           Exact filename match (e.g. .env)
  --depth <n>             Max recursion depth
  --open                  Prompt to open result dir in Finder after scan
  -v, --verbose           Show individual files under each directory
  --save <name>           Save current flags as a named preset
  --run <name>            Re-run a saved preset
  --history               Show last 5 searches
```

## File Type Presets

| Preset    | Extensions |
|-----------|-----------|
| `secrets` | `.env` `.key` `.pem` `.p12` `.pfx` `.secret` |
| `configs` | `.yaml` `.yml` `.toml` `.json` `.ini` `.conf` |
| `code`    | `.rs` `.ts` `.tsx` `.py` `.go` `.js` |
| `all`     | no filter |

Custom: pass comma-separated extensions — `--type .env,.toml`

## Presets & History

```bash
# Save a named preset
dirtrack /Volumes/WS4TB --since 1d --type secrets --save daily

# Re-run it later
dirtrack --run daily

# See last 5 searches (saved automatically in interactive mode)
dirtrack --history
```

Presets and history persist at `~/.config/dirtrack/history.json`.

## Automatically Skipped

`target/`, `.git/`, `node_modules/`, `.next/`, `__pycache__/`

## Performance

Scans ~14.5M files in ~60 seconds on a large workspace drive (macOS, external HDD).

## Limitations

- macOS only (uses `open` for Finder integration)
- Uses `mtime` — a moved/copied file retains its original modification time
- Cannot distinguish "modified" from "added" without a baseline snapshot
