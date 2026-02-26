<div align="center">

```
  ▲ Archivum
```

**Deterministic · Split · Checksummed · Compressed · Verifiable**

*Archive anything. Restore faithfully. Verify with confidence.*

---

[![Crates.io](https://img.shields.io/crates/v/archivum?style=for-the-badge&color=00d4ff&logo=rust)](https://crates.io/crates/archivum)
[![Downloads](https://img.shields.io/crates/d/archivum?style=for-the-badge&color=00d4ff)](https://crates.io/crates/archivum)
[![CI](https://img.shields.io/github/actions/workflow/status/ankit-chaubey/Archivum/ci.yml?style=for-the-badge&label=CI&logo=github)](https://github.com/ankit-chaubey/Archivum/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org)

</div>

---

## What is Archivum?

**Archivum** is a modern, fast, and trustworthy command-line archive system built in Rust. It goes far beyond `tar` and `zip` — every file is checksummed with SHA-256, the index is human-readable JSON with a Blake3 integrity seal, and the archive can be split into any size, compressed with 5 algorithms, searched, diffed, deduplicated, merged, updated incrementally, and repaired.

Designed for **long-term backups**, **forensic preservation**, **offline cold storage**, and **DevOps workflows** where correctness is non-negotiable.

---

## Feature Comparison

| Feature | tar | zip | rsync | **Archivum** |
|---------|:---:|:---:|:-----:|:------------:|
| Split into custom-size parts | ✗ | ✗ | ✗ | ✅ |
| Split by file count | ✗ | ✗ | ✗ | ✅ |
| Human-readable JSON index | ✗ | ✗ | ✗ | ✅ |
| Blake3-sealed index integrity | ✗ | ✗ | ✗ | ✅ |
| Per-file SHA-256 checksums | ✗ | CRC-32 | ✗ | ✅ |
| 5 compression algorithms | ✗ | deflate | ✗ | ✅ |
| Content deduplication | ✗ | ✗ | ✓ | ✅ |
| Incremental update | ✗ | ✗ | ✓ | ✅ |
| Drift detection (`diff`) | ✗ | ✗ | ✓ | ✅ |
| Archive merging | ✗ | ✗ | ✗ | ✅ |
| Smart pruning | ✗ | ✗ | ✗ | ✅ |
| Auto repair | ✗ | ✗ | ✗ | ✅ |
| File search (glob + substring) | ✗ | ✗ | ✗ | ✅ |
| Archive statistics | ✗ | ✗ | ✗ | ✅ |
| Stream single file to stdout | ✗ | ✗ | ✗ | ✅ |
| Shell completions | ✗ | ✗ | ✗ | ✅ |
| JSON output for scripting | ✗ | ✗ | ✗ | ✅ |
| Restore single file (`extract`) | ✗ | ✓ | ✗ | ✅ |
| Symlink preservation | varies | ✗ | ✓ | ✅ |
| Dry-run mode | ✗ | ✗ | ✓ | ✅ |
| Quiet + log-file mode | ✗ | ✗ | ✗ | ✅ |

---

## Installation

### From crates.io *(recommended)*

```bash
cargo install archivum
```

### From source

```bash
git clone https://github.com/ankit-chaubey/Archivum
cd Archivum
cargo build --release
# Binary: ./target/release/archivum
sudo cp target/release/archivum /usr/local/bin/
```

### Pre-built binaries

Download the latest binary from the [Releases page](https://github.com/ankit-chaubey/Archivum/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `archivum-linux-x86_64` |
| Linux aarch64 | `archivum-linux-aarch64` |
| macOS x86_64 | `archivum-macos-x86_64` |
| macOS Apple Silicon | `archivum-macos-aarch64` |
| Windows x86_64 | `archivum-windows-x86_64.exe` |

### Shell completions

```bash
# Bash
archivum completions bash >> ~/.bashrc

# Zsh
archivum completions zsh >> ~/.zshrc

# Fish
archivum completions fish > ~/.config/fish/completions/archivum.fish
```

---

## Quick Start

```bash
# Create an archive (zstd compression, 2 GB parts)
archivum create ./my-project ./backup --compress zstd --split-gb 2

# List contents
archivum list ./backup/index.arc.json

# Verify integrity
archivum verify ./backup/index.arc.json

# Restore
archivum restore ./backup/index.arc.json ./restored

# Diff — what changed since the archive was made?
archivum diff ./backup/index.arc.json ./my-project

# Search for files
archivum search ./backup/index.arc.json "*.rs"

# View stats
archivum stats ./backup/index.arc.json
```

---

## Commands

### `create` — Create an archive

```
archivum create <SOURCE> <OUTPUT> [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--compress <ALGO>` | `none` \| `gzip` \| `zstd` \| `bzip2` \| `lz4` | `none` |
| `--zstd-level <N>` | Zstd compression level (1–22) | `3` |
| `--split-gb <GB>` | Max size of each part in GB | `4` |
| `--split-files <N>` | Max files per part | `0` (unlimited) |
| `--exclude <GLOB>` | Exclude pattern (repeatable) | — |
| `--dedup` | Skip duplicate files (SHA-256 based) | off |
| `--notes <TEXT>` | Attach a note to the archive | — |
| `--threads <N>` | Checksum parallelism | `4` |
| `--dry-run` | Show what would happen without writing | off |
| `--quiet` | Suppress all output | off |
| `--log-file <PATH>` | Append output to a log file | — |

```bash
# Zstd compression, 1 GB parts, exclude build artifacts
archivum create ./my-project ./backup \
  --compress zstd --split-gb 1 \
  --exclude "target/**" --exclude "*.log"

# Deduplicate, annotate, dry-run first
archivum create ./photos ./backup --dedup --notes "Family photos 2026" --dry-run

# Split by file count (max 500 files per part)
archivum create ./documents ./backup --split-files 500

# Log output for automation
archivum create ./data ./backup --compress lz4 --quiet --log-file /var/log/archivum.log
```

**Output structure:**
```
backup/
├── index.arc.json           ← JSON index (human-readable)
├── index.arc.json.b3        ← Blake3 integrity seal
├── data.part000.tar         ← Part 0
├── data.part001.tar.zst     ← Part 1 (compressed)
└── ...
```

---

### `list` — List contents

```
archivum list <INDEX> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show all file entries |
| `--filter <GLOB>` | Filter entries by glob |
| `--json` | Output as JSON |

```bash
archivum list ./backup/index.arc.json
archivum list ./backup/index.arc.json --verbose --filter "**/*.rs"
archivum list ./backup/index.arc.json --json | jq '.header'
```

---

### `restore` — Restore an archive

```
archivum restore <INDEX> <TARGET> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--filter <GLOB>` | Only restore matching files |
| `-f, --force` | Overwrite existing files |
| `--restore-permissions` | Restore Unix file permissions |
| `--dry-run` | Show what would be restored |

```bash
# Full restore
archivum restore ./backup/index.arc.json ./restored

# Restore only Rust source files
archivum restore ./backup/index.arc.json ./restored --filter "**/*.rs"

# Force overwrite
archivum restore ./backup/index.arc.json ./restored --force --restore-permissions
```

> **Efficiency**: The restore engine groups files by tar part so each part is read exactly once — O(n + m) instead of the naïve O(n × m).

---

### `verify` — Verify integrity

```
archivum verify <INDEX> [OPTIONS]
```

Checks:
1. Blake3 seal on the index file (tamper detection)
2. All expected tar parts are present
3. Every file's SHA-256 matches the stored value

```bash
archivum verify ./backup/index.arc.json
archivum verify ./backup/index.arc.json --continue-on-error --json
```

---

### `diff` — Detect drift

```
archivum diff <INDEX> <SOURCE> [OPTIONS]
```

Compare an archive against the live source directory. Detects **added**, **removed**, and **modified** files.

| Option | Description |
|--------|-------------|
| `--changed-only` | Suppress unchanged files |
| `--checksum` | Use SHA-256 comparison (not just mtime/size) |
| `--json` | Output as JSON |

```bash
archivum diff ./backup/index.arc.json ./my-project --changed-only

# Output:
#   ~ MODIFIED  src/main.rs    (12.4 KB → 13.1 KB)
#   + ADDED     src/new_mod.rs (2.1 KB)
#   - REMOVED   old_file.txt
#   Added: 1  Removed: 1  Modified: 1  Unchanged: 142
```

---

### `search` — Search files

```
archivum search <INDEX> <PATTERN>
```

Search by glob pattern (`*.rs`, `**/*.jpg`) or substring (`config`, `2026`).

```bash
archivum search ./backup/index.arc.json "*.toml"
archivum search ./backup/index.arc.json "config" --json
```

---

### `stats` — Archive statistics

```
archivum stats <INDEX>
```

Displays compression ratio, size breakdown by file extension, part distribution, and deduplication savings.

```bash
archivum stats ./backup/index.arc.json
archivum stats ./backup/index.arc.json --json
```

---

### `info` — File metadata

```
archivum info <INDEX> <FILE>
```

```bash
archivum info ./backup/index.arc.json src/main.rs
```

Output:
```
──────────────────────────────────────────────────
  File:    src/main.rs
  Type:    file
  Size:    13.1 KiB
  SHA-256: e3b0c442...
  Part:    part000
  Modified: 2026-02-26 12:00:00 UTC
  Mode:    0o644
──────────────────────────────────────────────────
```

---

### `extract` — Extract single file

```
archivum extract <INDEX> <FILE> [OPTIONS]
```

```bash
archivum extract ./backup/index.arc.json src/main.rs
archivum extract ./backup/index.arc.json docs/report.pdf --output ./recovered.pdf
```

---

### `cat` — Stream file to stdout

```
archivum cat <INDEX> <FILE>
```

Stream a single file's content directly to stdout — perfect for piping.

```bash
archivum cat ./backup/index.arc.json config.toml
archivum cat ./backup/index.arc.json data.csv | wc -l
archivum cat ./backup/index.arc.json script.sh | bash
```

---

### `update` — Incremental update

```
archivum update <OLD_INDEX> <SOURCE> <OUTPUT> [OPTIONS]
```

Creates a new archive containing only files that are **new or modified** since the last archive. Unchanged files are referenced, not re-archived.

```bash
archivum update ./backup/index.arc.json ./my-project ./backup-v2
archivum update ./backup/index.arc.json ./src ./backup-v2 --checksum
```

---

### `merge` — Merge archives

```
archivum merge <INDEX1> <INDEX2> ... <OUTPUT>
```

Combine multiple archives into a single unified archive.

```bash
archivum merge ./backup-jan/index.arc.json ./backup-feb/index.arc.json ./merged \
  --compress zstd
```

---

### `prune` — Prune old archives

```
archivum prune <DIR> [OPTIONS]
```

Remove old archive directories, keeping the most recent N or those newer than a time threshold.

| Option | Description | Default |
|--------|-------------|---------|
| `--keep <N>` | Keep the N most recent archives | `3` |
| `--max-age <DAYS>` | Remove archives older than N days | `0` (off) |
| `--dry-run` | Show what would be removed | off |

```bash
archivum prune /backups --keep 5
archivum prune /backups --max-age 90 --dry-run
```

---

### `repair` — Repair a corrupted index

```
archivum repair <DIR>
```

Rebuilds a missing or corrupted `index.arc.json` by scanning the tar parts on disk.

```bash
archivum repair ./backup
archivum repair ./backup --compression zstd
```

---

### `completions` — Shell completions

```
archivum completions <SHELL>
```

```bash
archivum completions bash
archivum completions zsh
archivum completions fish
```

---

## Configuration

Archivum reads a `config.toml` from:
- **Linux/macOS**: `~/.config/archivum/config.toml`
- **Windows**: `%APPDATA%\archivum\config.toml`

```bash
# Interactive setup wizard
archivum setup

# Show current configuration
archivum config
```

**Example `config.toml`:**

```toml
[defaults]
compress   = "zstd"
zstd_level = 9
split_gb   = 2.0
split_files = 0
threads    = 8

[output]
quiet = false
json  = false

[create]
dedup   = false
notes   = ""
exclude = ["**/.DS_Store", "**/Thumbs.db", "**/__pycache__/**"]

[restore]
force               = false
restore_permissions = true

[update]
checksum_diff = false

[prune]
keep_last    = 5
max_age_days = 0
```

See [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) for full reference.

---

## Index Format

The `index.arc.json` is intentionally human-readable — inspect with any editor, `jq`, or `grep` without Archivum installed.

```json
{
  "header": {
    "version": 3,
    "created_at_unix": 1740567000,
    "created_at_human": "2026-02-26 12:30:00 UTC",
    "total_files": 142,
    "total_dirs": 18,
    "total_symlinks": 3,
    "total_size": 52428800,
    "total_parts": 2,
    "compression": "zstd",
    "zstd_level": 9,
    "notes": "Production backup — pre-deploy"
  },
  "entries": [
    {
      "path": "src/main.rs",
      "entry_type": "file",
      "size": 13421,
      "mtime": 1740500000,
      "unix_mode": 33188,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb924...",
      "tar_part": 0,
      "dedup_of": null,
      "symlink_target": null
    }
  ]
}
```

The accompanying `.b3` file contains a Blake3 hash of the index — `verify` checks this automatically to detect any tampering.

See [`docs/INDEX_FORMAT.md`](docs/INDEX_FORMAT.md) for the full schema reference.

---

## Compression Algorithms

| Algorithm | Flag | Speed | Ratio | Best for |
|-----------|------|:-----:|:-----:|----------|
| None | `none` | ⚡⚡⚡ | 1× | Media, already-compressed files |
| LZ4 | `lz4` | ⚡⚡⚡ | ~1.5× | Real-time, fast storage |
| Gzip | `gzip` | ⚡⚡ | ~2–4× | Universal compatibility |
| Zstd | `zstd` | ⚡⚡ | ~3–7× | Best all-round choice |
| Bzip2 | `bzip2` | ⚡ | ~3–6× | High-ratio, space-critical |

---

## Global Flags

These flags work with every command:

| Flag | Description |
|------|-------------|
| `--quiet` | Suppress all stdout output |
| `--json` | Output machine-readable JSON |
| `--dry-run` | Simulate without writing anything |
| `--log-file <PATH>` | Append all output to a file |

---

## Architecture

```
src/
├── main.rs         — CLI (clap), subcommand dispatch, OutputCtx wiring
├── output.rs       — OutputCtx: quiet / json / dry-run / log-file
├── config.rs       — config.toml loading, setup wizard
├── scan.rs         — Directory traversal, symlink detection, excludes
├── checksum.rs     — Parallel SHA-256 + Blake3 computation
├── compress.rs     — Compression abstraction (none/gzip/zstd/bzip2/lz4)
├── tar_writer.rs   — Two-pass tar part assignment + writing
├── index.rs        — ArchivumIndex v3: build, read, write, print, Blake3 seal
├── restore.rs      — Grouped restore + single-file extract, path traversal guard
├── verify.rs       — Part existence + checksum + Blake3 index verification
├── diff.rs         — Archive vs source drift detection
├── search.rs       — Glob + substring search
├── stats.rs        — Compression ratio, extension breakdown, dedup savings
├── update.rs       — Incremental archive update
├── merge.rs        — Multi-archive merge
├── prune.rs        — Age + count-based archive pruning
├── repair.rs       — Index reconstruction from tar parts
├── cat.rs          — Stream single file to stdout
├── completions.rs  — Shell completion generation
└── utils.rs        — Formatting, timestamps, banner
```

---

## Performance

- **Parallel checksums**: SHA-256 computed with a configurable Rayon thread pool (`--threads`)
- **Efficient restore**: Files grouped by tar part — each part opened exactly once
- **Streaming writes**: Source → tar with no intermediate buffering
- **Deduplication**: Skips re-writing files with identical SHA-256 hashes
- **Incremental update**: Only archives new/modified files, O(diff) not O(total)

---

## Global Flags

All commands support these flags for automation:

```bash
archivum --quiet create ./src ./backup --compress zstd
archivum --json  list ./backup/index.arc.json | jq '.entries | length'
archivum --log-file /var/log/arc.log verify ./backup/index.arc.json
archivum --dry-run restore ./backup/index.arc.json ./out
```

---

## Exclude Patterns

Uses [glob](https://en.wikipedia.org/wiki/Glob_(programming)) syntax via `globset`:

```bash
archivum create ./src ./out \
  --exclude "target/**"          \  # Rust build dir
  --exclude "node_modules/**"    \  # Node.js packages
  --exclude "**/.DS_Store"       \  # macOS metadata
  --exclude "**/*.tmp"           \  # Temp files
  --exclude "**/__pycache__/**"     # Python cache
```

---

## Automation Examples

```bash
# Daily backup cron — quiet, logged
0 2 * * * archivum create /data /backups/$(date +%F) \
  --compress zstd --dedup --quiet --log-file /var/log/archivum.log

# Verify backup integrity weekly
0 6 * * 1 archivum verify /backups/$(ls -t /backups | head -1)/index.arc.json \
  --json > /var/log/archivum-verify.json

# Prune old backups (keep last 14)
archivum prune /backups --keep 14

# CI pre-deploy archive
archivum create ./dist ./releases/v${VERSION} --compress zstd \
  --notes "Release ${VERSION}" --quiet

# Pipe a config file from archive
archivum cat ./backup/index.arc.json etc/nginx.conf | diff - /etc/nginx/nginx.conf
```

---

## Contributing

Contributions are welcome! See [`CONTRIBUTING.md`](CONTRIBUTING.md) for guidelines.

```bash
git clone https://github.com/ankit-chaubey/Archivum
cd Archivum
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt
```

---

## Security

To report a vulnerability, see [`SECURITY.md`](SECURITY.md).

---

## License

Apache 2.0 — see [LICENSE](LICENSE)

```
Copyright 2026 Ankit Chaubey

Licensed under the Apache License, Version 2.0
```

---

<div align="center">

**▲ Archivum v0.2.0**

Made with ♥ by [Ankit Chaubey](https://github.com/ankit-chaubey) · [ankitchaubey.dev@gmail.com](mailto:ankitchaubey.dev@gmail.com)

[crates.io](https://crates.io/crates/archivum) · [docs](docs/) · [changelog](CHANGELOG.md) · [issues](https://github.com/ankit-chaubey/Archivum/issues)

</div>
