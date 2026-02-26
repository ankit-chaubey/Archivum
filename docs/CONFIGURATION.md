# Archivum — Configuration Reference

Archivum reads a `config.toml` file from:

| Platform | Path |
|----------|------|
| Linux / macOS | `~/.config/archivum/config.toml` |
| Windows | `%APPDATA%\archivum\config.toml` |

Run `archivum setup` for an interactive wizard, or create the file manually.

---

## Full Reference

```toml
# ─────────────────────────────────────────────────────────
# Archivum Configuration — v0.2.0
# ~/.config/archivum/config.toml
# ─────────────────────────────────────────────────────────

[defaults]
# Default compression algorithm for create and update
# Options: none | gzip | zstd | bzip2 | lz4
compress = "zstd"

# Zstd compression level (1 = fastest, 22 = smallest)
zstd_level = 9

# Maximum size of each tar part in gigabytes
split_gb = 2.0

# Maximum files per part (0 = unlimited)
split_files = 0

# Number of threads for parallel SHA-256 checksums
threads = 8

# ─────────────────────────────────────────────────────────
[output]
# Suppress all stdout output globally
quiet = false

# Output JSON for all commands globally
json = false

# ─────────────────────────────────────────────────────────
[create]
# Enable deduplication by default
dedup = false

# Default note to attach to archives (empty = disabled)
notes = ""

# Global exclude patterns — applied to every create/update
exclude = [
  "**/.DS_Store",
  "**/Thumbs.db",
  "**/__pycache__/**",
  "**/*.pyc",
  "**/.git/**",
]

# ─────────────────────────────────────────────────────────
[restore]
# Overwrite existing files without prompting
force = false

# Restore Unix file permissions (chmod bits)
restore_permissions = true

# ─────────────────────────────────────────────────────────
[update]
# Use SHA-256 (not just mtime+size) for change detection in diff/update
checksum_diff = false

# ─────────────────────────────────────────────────────────
[prune]
# Number of most-recent archives to keep
keep_last = 5

# Remove archives older than this many days (0 = disabled)
max_age_days = 0
```

---

## Precedence

Command-line flags always override `config.toml`. Config always overrides compiled defaults.

```
CLI flags  >  config.toml  >  built-in defaults
```

---

## Environment

No environment variables are currently used. All configuration is via the file and CLI flags.

---

## Example Configurations

### Developer workstation

```toml
[defaults]
compress   = "zstd"
zstd_level = 6
split_gb   = 4.0
threads    = 8

[create]
exclude = [
  "target/**", "node_modules/**", ".git/**",
  "**/*.log", "**/.DS_Store",
]

[prune]
keep_last = 10
```

### Server / automation

```toml
[defaults]
compress   = "zstd"
zstd_level = 19
split_gb   = 50.0
threads    = 16

[output]
quiet = true

[create]
dedup = true

[prune]
keep_last    = 30
max_age_days = 90
```

### Minimal / portable

```toml
[defaults]
compress  = "none"
split_gb  = 4.0
threads   = 2
```
