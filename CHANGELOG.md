# Changelog

All notable changes to Archivum are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).  
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] — 2026-02-26

### 🚀 New Features

- **`search` command** — Find files by glob (`*.rs`) or substring (`config`)
- **`stats` command** — Compression ratio, extension breakdown, dedup savings, per-part sizes
- **`cat` command** — Stream any archived file directly to stdout
- **`update` command** — Incremental archive: only re-archives new/modified files
- **`merge` command** — Combine multiple archives into one unified archive
- **`prune` command** — Remove old archives by count (`--keep N`) or age (`--max-age DAYS`)
- **`repair` command** — Rebuild a corrupted/missing `index.arc.json` from tar parts
- **`completions` command** — Generate shell completions (bash / zsh / fish)
- **`setup` command** — Interactive `config.toml` setup wizard
- **`config` command** — Display current effective configuration
- **5 compression algorithms**: `none`, `gzip`, `zstd`, `bzip2`, `lz4`
- **Content deduplication** (`--dedup`) — SHA-256 based, skips identical files
- **Archive notes** (`--notes`) — Attach a human-readable note to any archive
- **Split by file count** (`--split-files N`) — complement to `--split-gb`
- **Blake3 index integrity seal** — `index.arc.json.b3` tamper detection
- **`OutputCtx`** — Unified quiet/json/dry-run/log-file output system across all commands
- **`--quiet` flag** — Suppress all stdout output
- **`--json` flag** — Machine-readable JSON output for every command
- **`--dry-run` flag** — Safe simulation mode: no files written
- **`--log-file` flag** — Append structured output to a file
- **Configuration file** (`~/.config/archivum/config.toml`) with all defaults
- **`arc` alias** — Short alias for the `archivum` binary
- **Index v3** — Adds `notes`, `dedup_of`, `zstd_level`, multi-base part refs, Blake3 seal
- **Parallel checksums** via Rayon thread pool

### 🐛 Fixed

- Search: glob patterns like `sub` incorrectly matched only exact strings, not substrings  
- Quiet mode: `--quiet` flag was ignored in `list` and banner output  
- Log file: log file was empty because `list` used raw `println!` bypassing `OutputCtx`  
- Create: nonexistent source directory produced success exit code instead of error  
- Roundtrip test: md5sum comparison included file paths, causing false content-differs failures  

### ⚡ Improved

- Restore engine: O(n + m) grouping by tar part (was O(n × m))
- All commands respect `--quiet`, `--json`, `--log-file` consistently
- Print banner and print_summary both route through `OutputCtx`
- Richer colored terminal output throughout

### 🔒 Security

- Path traversal guard in restore (`..` components rejected)
- Blake3 seal on index file — tampering is detected before restore/verify
- Apache 2.0 license headers in all source files

---

## [0.1.0] — 2024

### Added

- **SHA-256 checksums** for every archived file
- **Multi-algorithm compression**: `none`, `gzip`, `zstd`
- **Parallel checksum computation** with `--threads`
- **`verify` command** — part existence + checksum verification
- **`diff` command** — detect added/removed/modified files vs source
- **`info` command** — full metadata for a single file
- **`extract` command** — restore a single file without full restore
- **Symlink preservation**
- **Glob-based exclude patterns** (`--exclude "*.log"`)
- **Filter patterns** in `list` and `restore`
- **Rich CLI** with clap, colored output, progress bars
- **GitHub Actions**: CI (Linux/macOS/Windows), publish workflow, security audit
- **Index v2** — adds `created_at_human`, `total_symlinks`, `total_parts`, `compression`, `sha256`, `symlink_target`
- **Two-pass tar writing**
- **Efficient restore** grouped by tar part
- `--force` flag for restore
- `--restore-permissions` flag
- `--verbose` flag for `list`

### Fixed

- Panic on missing CLI arguments
- Inefficient restore re-opening tar per file
- Root directory included as empty entry in scan
- No cross-platform timestamp support

---

[0.2.0]: https://github.com/ankit-chaubey/Archivum/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ankit-chaubey/Archivum/releases/tag/v0.1.0
