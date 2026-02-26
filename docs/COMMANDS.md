# Archivum — Command Reference

> Full reference for every `archivum` subcommand.

---

## Global Options

These apply to **every** command:

| Flag | Description |
|------|-------------|
| `--quiet` | Suppress all stdout output (errors still go to stderr) |
| `--json` | Output machine-readable JSON |
| `--dry-run` | Simulate — nothing is written to disk |
| `--log-file <PATH>` | Append all output (no ANSI colour) to a file |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

---

## `create`

Create a new archive from a source directory.

```
archivum create <SOURCE> <o> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `SOURCE` | Source directory to archive |
| `OUTPUT` | Output directory (will be created if absent) |

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--compress` | `none`\|`gzip`\|`zstd`\|`bzip2`\|`lz4` | `none` | Compression algorithm |
| `--zstd-level` | 1–22 | `3` | Zstd compression level |
| `--split-gb` | float | `4.0` | Max size per part in GB |
| `--split-files` | int | `0` | Max files per part (0 = unlimited) |
| `--exclude` | glob | — | Exclude pattern (repeatable) |
| `--dedup` | flag | off | Skip files with duplicate SHA-256 |
| `--notes` | string | — | Attach a note to the archive header |
| `--threads` | int | `4` | Checksum thread count |
| `--dry-run` | flag | off | Simulate without writing |
| `--quiet` | flag | off | Suppress output |
| `--log-file` | path | — | Log file path |

### Output

```
<output>/
├── index.arc.json           ← JSON index (all metadata + checksums)
├── index.arc.json.b3        ← Blake3 integrity seal
├── data.part000.tar         ← Part 0 (uncompressed)
├── data.part000.tar.gz      ← Part 0 (gzip)
├── data.part000.tar.zst     ← Part 0 (zstd)
├── data.part000.tar.bz2     ← Part 0 (bzip2)
├── data.part000.tar.lz4     ← Part 0 (lz4)
└── data.part001.tar.*       ← Part 1 (if split)
```

---

## `list`

List the contents of an archive.

```
archivum list <INDEX> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show all file entries with type, size, part |
| `--filter <GLOB>` | Only show matching entries |
| `--json` | Output as JSON |

---

## `restore`

Restore all or part of an archive to a target directory.

```
archivum restore <INDEX> <TARGET> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--filter <GLOB>` | Only restore matching files |
| `-f, --force` | Overwrite existing files |
| `--restore-permissions` | Restore Unix `chmod` bits |
| `--dry-run` | Show what would be restored |

---

## `verify`

Verify archive integrity (Blake3 seal + SHA-256 checksums).

```
archivum verify <INDEX> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-c, --continue-on-error` | Don't stop on first failure |
| `--json` | Output results as JSON |

Checks performed:
1. Blake3 seal on `index.arc.json` (tamper detection)
2. All expected tar parts present on disk
3. Every file's SHA-256 matches stored value

Exit code `0` = PASS, `1` = FAIL or CORRUPT.

---

## `diff`

Compare an archive against its source directory.

```
archivum diff <INDEX> <SOURCE> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--changed-only` | Only show changed/added/removed files |
| `--checksum` | Use SHA-256 instead of mtime+size for change detection |
| `--json` | Output as JSON |

Status codes in output:
- `+` ADDED — file exists in source but not in archive
- `-` REMOVED — file in archive but not in source
- `~` MODIFIED — size or mtime differ (or SHA-256 if `--checksum`)
- `·` UNCHANGED

---

## `search`

Search the archive index.

```
archivum search <INDEX> <PATTERN>
```

- If pattern contains `*`, `?`, or `[` → glob match
- Otherwise → case-insensitive substring match

```bash
archivum search ./backup/index.arc.json "*.rs"       # glob
archivum search ./backup/index.arc.json "config"     # substring
archivum search ./backup/index.arc.json "src/main"   # path substring
```

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |

---

## `stats`

Show archive statistics.

```
archivum stats <INDEX>
```

Displays:
- Compression ratio (original vs compressed size)
- File count by extension (top 10)
- Per-part size breakdown
- Deduplication savings

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |

---

## `info`

Show detailed metadata for a single file.

```
archivum info <INDEX> <FILE>
```

```bash
archivum info ./backup/index.arc.json src/main.rs
archivum info ./backup/index.arc.json src/main.rs --json
```

---

## `extract`

Extract a single file to disk.

```
archivum extract <INDEX> <FILE> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--output <PATH>` | Write to this path (default: file name in current dir) |

---

## `cat`

Stream a file's content to stdout.

```
archivum cat <INDEX> <FILE>
```

Useful for piping without extracting to disk:
```bash
archivum cat ./backup/index.arc.json config.toml | grep "key ="
archivum cat ./backup/index.arc.json data.csv | python3 process.py
```

---

## `update`

Incremental update — only re-archives new or modified files.

```
archivum update <OLD_INDEX> <SOURCE> <o> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--compress` | Compression for new parts |
| `--split-gb` | Part size for new parts |
| `--checksum` | Use SHA-256 (not just mtime) for change detection |
| `--threads` | Checksum thread count |

---

## `merge`

Merge multiple archives into one.

```
archivum merge <INDEX> [<INDEX>...] <o>
```

```bash
archivum merge ./jan/index.arc.json ./feb/index.arc.json ./merged --compress zstd
```

---

## `prune`

Remove old archive directories.

```
archivum prune <DIR> [OPTIONS]
```

Looks for subdirectories of `<DIR>` that contain `index.arc.json`.

| Option | Default | Description |
|--------|---------|-------------|
| `--keep <N>` | `3` | Keep the N most recent archives |
| `--max-age <DAYS>` | `0` | Remove archives older than N days (0 = off) |
| `--dry-run` | off | Show what would be removed |

---

## `repair`

Rebuild a corrupted or missing `index.arc.json` by scanning tar parts.

```
archivum repair <DIR> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--compression <ALGO>` | Compression of the existing parts |

---

## `completions`

Generate shell tab-completion scripts.

```
archivum completions <SHELL>
```

Supported: `bash`, `zsh`, `fish`

```bash
archivum completions bash >> ~/.bashrc && source ~/.bashrc
archivum completions zsh  >> ~/.zshrc  && source ~/.zshrc
archivum completions fish > ~/.config/fish/completions/archivum.fish
```

---

## `setup`

Interactive wizard to create `~/.config/archivum/config.toml`.

```
archivum setup
```

---

## `config`

Display the current effective configuration.

```
archivum config
```
