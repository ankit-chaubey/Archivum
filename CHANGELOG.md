# Changelog

All notable changes to Archivum are documented here.

## [2.0.0] — 2024

### Added
- **SHA-256 checksums** for every archived file — stored in `index.arc.json`
- **Multi-algorithm compression**: `none`, `gzip`, `zstd`
- **Parallel checksum computation** with `--threads`
- **`verify` command** — checks part existence and all file checksums
- **`diff` command** — detects added/removed/modified files vs source
- **`info` command** — shows full metadata for a single file
- **`extract` command** — restores a single file without restoring the whole archive
- **Symlink preservation** — symlinks recorded in index, restored on Unix
- **Glob-based exclude patterns** (`--exclude "*.log"`) via `globset`
- **Filter patterns** in `list`, `restore` — e.g. `--filter "**/*.jpg"`
- **Rich CLI** with `clap`, colored output, progress bars via `indicatif`
- **GitHub Actions**: CI (Linux/macOS/Windows), publish workflow, security audit
- **`index.arc.json` v2** — adds `created_at_human`, `total_symlinks`, `total_parts`, `compression`, `sha256`, `symlink_target`
- **Two-pass tar writing** — assigns parts in pass 1, writes in pass 2 (no borrow issues)
- **Efficient restore** — groups files by tar part, O(n+m) instead of O(n×m)
- `--force` flag for restore (overwrite existing)
- `--restore-permissions` flag for Unix permissions
- `--verbose` flag for `list`

### Fixed
- Panic on missing CLI arguments (now handled by clap)
- Inefficient restore that re-opened tar for every file
- Missing bounds checking on `a[2]`, `a[3]` in main
- Root directory included as empty entry in scan
- No cross-platform timestamp support

### Changed
- Minimum Rust edition: 2021 (unchanged)
- Version bumped to 2.0.0 to reflect breaking index format change

## [1.0.0] — Initial release

- Basic `create`, `list`, `restore` commands
- Split tar archives with `index.arc.json`
