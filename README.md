<p align="center">
  <img src="https://img.shields.io/crates/v/archivum?style=for-the-badge&color=cyan" alt="crates.io version" />
  <img src="https://img.shields.io/crates/d/archivum?style=for-the-badge&color=cyan" alt="downloads" />
  <img src="https://img.shields.io/github/actions/workflow/status/ankit-chaubey/Archivum/ci.yml?style=for-the-badge&label=CI" alt="CI" />
  <img src="https://img.shields.io/github/license/ankit-chaubey/Archivum?style=for-the-badge&color=blue" alt="MIT license" />
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange?style=for-the-badge&logo=rust" alt="Rust 1.75+" />
</p>

<h1 align="center">▲ Archivum</h1>
<p align="center"><strong>Deterministic · Split · Checksummed · Compressed archive system</strong></p>
<p align="center">Archive anything. Restore faithfully. Verify with confidence.</p>

---

## Overview

**Archivum** is a command-line tool for creating deterministic, split, checksummed, and optionally compressed archives of directories. It is designed for **long-term backups**, **forensic preservation**, and **offline storage** — where correctness and verifiability matter more than speed.

### Why Archivum?

| Feature | tar | zip | Archivum |
|---------|-----|-----|----------|
| Split into parts | ✗ | ✗ | ✅ |
| Human-readable index | ✗ | ✗ | ✅ |
| Per-file SHA-256 checksums | ✗ | CRC-32 | ✅ |
| Drift detection (`diff`) | ✗ | ✗ | ✅ |
| Multi-compression (gzip/zstd) | ✗ | deflate | ✅ |
| Restore single file | ✗ | ✓ | ✅ |
| Symlink preservation | varies | ✗ | ✅ |
| Progress bars | ✗ | ✗ | ✅ |

---

## Installation

### From crates.io

```bash
cargo install archivum
```

### From source

```bash
git clone https://github.com/ankit-chaubey/Archivum
cd Archivum
cargo build --release
# Binary at ./target/release/archivum
```

### Pre-built binaries

Download from the [Releases page](https://github.com/ankit-chaubey/Archivum/releases) for:
- Linux x86_64 / aarch64
- macOS x86_64 / Apple Silicon
- Windows x86_64

---

## Quick Start

```bash
# 1. Create an archive (4 GB parts, no compression)
archivum create ./my-photos ./backup

# 2. Create with compression and smaller parts
archivum create ./my-photos ./backup --compress zstd --split-gb 1

# 3. Exclude patterns
archivum create ./my-project ./backup --exclude "*.log" --exclude "target/**"

# 4. List the archive contents
archivum list ./backup/index.arc.json

# 5. List with full file details
archivum list ./backup/index.arc.json --verbose

# 6. Verify integrity (checksums)
archivum verify ./backup/index.arc.json

# 7. Restore everything
archivum restore ./backup/index.arc.json ./restored

# 8. Restore only specific files
archivum restore ./backup/index.arc.json ./restored --filter "**/*.jpg"

# 9. Extract a single file
archivum extract ./backup/index.arc.json photos/2024/holiday.jpg

# 10. Diff archive vs source (detect drift)
archivum diff ./backup/index.arc.json ./my-photos
```

---

## Commands

### `create` — Create an archive

```
archivum create <SOURCE> <OUTPUT> [OPTIONS]

Arguments:
  SOURCE    Source directory to archive
  OUTPUT    Output directory for archive parts

Options:
  --split-gb <GB>         Max size of each tar part in gigabytes [default: 4]
  --compress <ALGO>       Compression: none | gzip | zstd [default: none]
  --exclude <PATTERN>     Exclude glob pattern (repeatable)
  --threads <N>           Checksum threads [default: 4]
```

**Examples:**
```bash
# Default: 4 GB uncompressed parts
archivum create /data/photos /mnt/backup/photos

# Zstd compression, 1 GB parts
archivum create /data/photos /mnt/backup/photos --compress zstd --split-gb 1

# Exclude build artifacts and logs
archivum create ./my-project ./archive \
  --exclude "target/**" \
  --exclude "**/*.log" \
  --exclude ".git/**"
```

**Output structure:**
```
backup/
├── index.arc.json        ← Human-readable index + checksums
├── data.part000.tar      ← First tar part
├── data.part001.tar      ← Second tar part (if split)
└── ...
```

---

### `list` — List archive contents

```
archivum list <INDEX> [OPTIONS]

Arguments:
  INDEX    Path to index.arc.json

Options:
  -v, --verbose           Show all file entries
  --filter <PATTERN>      Filter entries by glob pattern
```

**Examples:**
```bash
# Summary only
archivum list ./backup/index.arc.json

# All files, verbose
archivum list ./backup/index.arc.json --verbose

# Only .jpg files
archivum list ./backup/index.arc.json --verbose --filter "**/*.jpg"
```

---

### `restore` — Restore an archive

```
archivum restore <INDEX> <TARGET> [OPTIONS]

Arguments:
  INDEX    Path to index.arc.json
  TARGET   Target directory to restore into

Options:
  --filter <PATTERN>      Only restore matching files
  -f, --force             Overwrite existing files
  --restore-permissions   Restore Unix file permissions
```

The restore engine groups files by tar part to avoid re-reading tars, making it O(n + m) instead of the naïve O(n × m).

```bash
# Restore everything
archivum restore ./backup/index.arc.json ./restored

# Restore only images
archivum restore ./backup/index.arc.json ./restored --filter "**/*.{jpg,png,gif}"

# Force overwrite existing
archivum restore ./backup/index.arc.json ./restored --force
```

---

### `verify` — Verify integrity

```
archivum verify <INDEX> [OPTIONS]

Arguments:
  INDEX    Path to index.arc.json

Options:
  -c, --continue-on-error    Don't stop on first error
```

Verifies:
1. All expected tar parts are present on disk
2. Every file's SHA-256 checksum matches the stored value

```bash
archivum verify ./backup/index.arc.json

# Don't abort on first error
archivum verify ./backup/index.arc.json --continue-on-error
```

---

### `diff` — Detect drift

```
archivum diff <INDEX> <SOURCE> [OPTIONS]

Arguments:
  INDEX    Path to index.arc.json
  SOURCE   Current source directory to compare against

Options:
  --changed-only    Only show changed/added/removed files
```

Shows files that have been **added**, **removed**, or **modified** since the archive was created.

```bash
archivum diff ./backup/index.arc.json ./my-photos
```

Output:
```
  + ADDED    photos/new-puppy.jpg (3.2 MB)
  - REMOVED  photos/old-cat.jpg
  ~ MODIFIED photos/vacation.jpg (4.1 MB → 5.3 MB)
  
  Added: 1  Removed: 1  Modified: 1  Unchanged: 3847
```

---

### `info` — File details

```
archivum info <INDEX> <FILE>
```

Prints detailed metadata for a single file in the archive.

```bash
archivum info ./backup/index.arc.json photos/holiday.jpg
```

---

### `extract` — Extract single file

```
archivum extract <INDEX> <FILE> [OPTIONS]

Options:
  --output <PATH>    Output path (default: current directory)
```

```bash
archivum extract ./backup/index.arc.json photos/holiday.jpg
archivum extract ./backup/index.arc.json docs/report.pdf --output ./recovered.pdf
```

---

## Index Format

The `index.arc.json` is a human-readable JSON file:

```json
{
  "header": {
    "version": 2,
    "created_at_unix": 1709123456,
    "created_at_human": "2024-02-28 12:30:56 UTC",
    "total_files": 3847,
    "total_dirs": 142,
    "total_symlinks": 5,
    "total_size": 14293847192,
    "total_parts": 4,
    "compression": "zstd"
  },
  "entries": [
    {
      "path": "photos/2024/holiday.jpg",
      "entry_type": "file",
      "size": 4194304,
      "mtime": 1709100000,
      "unix_mode": 33188,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "tar_part": 0,
      "symlink_target": null
    }
  ]
}
```

The index is intentionally human-readable so you can inspect it with any text editor, `jq`, or `grep` without needing Archivum installed.

---

## Compression

| Algorithm | Flag | Speed | Ratio | Best for |
|-----------|------|-------|-------|----------|
| None | `--compress none` | Fastest | 1× | Already-compressed media |
| Gzip | `--compress gzip` | Fast | ~2–5× | General use |
| Zstd | `--compress zstd` | Fast + good ratio | ~3–7× | Text, code, documents |

---

## Exclude Patterns

Uses [glob](https://en.wikipedia.org/wiki/Glob_(programming)) syntax:

```bash
# Exclude a directory
archivum create ./src ./out --exclude "node_modules/**"

# Exclude file type
archivum create ./src ./out --exclude "**/*.tmp"

# Multiple patterns
archivum create ./src ./out \
  --exclude "target/**" \
  --exclude "**/.DS_Store" \
  --exclude "**/__pycache__/**"
```

---

## Performance

- **Checksum parallelism**: SHA-256 computation uses configurable thread pool (`--threads`)
- **Efficient restore**: Files are grouped by tar part — each part is read exactly once
- **Streaming writes**: Files are streamed directly from source to tar without buffering everything in memory

---

## Architecture

```
src/
├── main.rs         — CLI (clap), subcommand dispatch
├── scan.rs         — Directory traversal, symlink detection, excludes
├── checksum.rs     — Parallel SHA-256 computation
├── compress.rs     — Compression abstraction (none/gzip/zstd)
├── tar_writer.rs   — Two-pass tar part assignment + writing
├── index.rs        — ArchivumIndex: build, read, write, print
├── restore.rs      — Efficient grouped restore + single-file extract
├── verify.rs       — Part existence + checksum verification
├── diff.rs         — Archive vs source drift detection
└── utils.rs        — Formatting, timestamps, banner
```

---

## Contributing

```bash
git clone https://github.com/ankit-chaubey/Archivum
cd Archivum
cargo test
cargo clippy -- -D warnings
cargo fmt
```

PRs are welcome! Please open an issue first for large changes.

---

## Publishing

Releases are automated via GitHub Actions:

1. Tag a release: `git tag v2.0.0 && git push origin v2.0.0`
2. CI runs tests on Linux, macOS, Windows
3. Release binaries are built for 5 platforms
4. Package is published to [crates.io/crates/archivum](https://crates.io/crates/archivum)

Add your `CARGO_REGISTRY_TOKEN` to repository secrets to enable publishing.

---

## License

MIT — see [LICENSE](LICENSE)

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/ankit-chaubey">Ankit Chaubey</a>
</p>
