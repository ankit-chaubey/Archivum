# Archivum — Index Format Reference

The `index.arc.json` file is the heart of every Archivum archive. It is a human-readable JSON file containing all metadata, checksums, and structural information needed to list, verify, restore, or search an archive.

---

## File Naming

```
<output>/
├── index.arc.json        ← The index
└── index.arc.json.b3     ← Blake3 integrity seal (hex string)
```

The `.b3` file contains the Blake3 hash of `index.arc.json`. The `verify` command checks this automatically to detect any modification or tampering.

---

## Schema (v3)

```json
{
  "header": {
    "version":          3,
    "created_at_unix":  1740567000,
    "created_at_human": "2026-02-26 12:30:00 UTC",
    "total_files":      142,
    "total_dirs":       18,
    "total_symlinks":   3,
    "total_size":       52428800,
    "total_parts":      2,
    "compression":      "zstd",
    "zstd_level":       9,
    "notes":            "Pre-deploy snapshot",
    "part_bases":       ["data"]
  },
  "entries": [
    {
      "path":           "src/main.rs",
      "entry_type":     "file",
      "size":           13421,
      "mtime":          1740500000,
      "unix_mode":      33188,
      "sha256":         "e3b0c44298fc1c149afbf4c8996fb924...",
      "tar_part":       0,
      "dedup_of":       null,
      "symlink_target": null
    },
    {
      "path":           "assets",
      "entry_type":     "directory",
      "size":           0,
      "mtime":          1740499000,
      "unix_mode":      16877,
      "sha256":         null,
      "tar_part":       0,
      "dedup_of":       null,
      "symlink_target": null
    },
    {
      "path":           "run.sh",
      "entry_type":     "symlink",
      "size":           0,
      "mtime":          1740498000,
      "unix_mode":      null,
      "sha256":         null,
      "tar_part":       0,
      "dedup_of":       null,
      "symlink_target": "scripts/run.sh"
    },
    {
      "path":           "src/copy_of_main.rs",
      "entry_type":     "file",
      "size":           13421,
      "mtime":          1740501000,
      "unix_mode":      33188,
      "sha256":         null,
      "tar_part":       0,
      "dedup_of":       "src/main.rs"
    }
  ]
}
```

---

## Header Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | int | Index format version (current: `3`) |
| `created_at_unix` | int | Archive creation timestamp (Unix epoch) |
| `created_at_human` | string | Human-readable timestamp (UTC) |
| `total_files` | int | Count of regular files |
| `total_dirs` | int | Count of directories |
| `total_symlinks` | int | Count of symbolic links |
| `total_size` | int | Sum of all file sizes in bytes |
| `total_parts` | int | Number of tar parts |
| `compression` | string | `none` \| `gzip` \| `zstd` \| `bzip2` \| `lz4` |
| `zstd_level` | int | Zstd level (only meaningful when compression = `zstd`) |
| `notes` | string | User-supplied annotation (may be empty) |
| `part_bases` | array | Base names for tar parts (usually `["data"]`) |

---

## Entry Fields

| Field | Type | Nullable | Description |
|-------|------|----------|-------------|
| `path` | string | No | Relative path from archive root |
| `entry_type` | string | No | `"file"` \| `"directory"` \| `"symlink"` |
| `size` | int | No | File size in bytes (0 for dirs and symlinks) |
| `mtime` | int | Yes | Last-modified timestamp (Unix epoch) |
| `unix_mode` | int | Yes | Unix permissions as decimal (e.g. `33188` = `0o100644`) |
| `sha256` | string | Yes | Hex SHA-256 of file content (null for dirs, symlinks, dedup entries) |
| `tar_part` | int | No | Zero-based index of the tar part containing this file |
| `dedup_of` | string | Yes | If set, this file is a duplicate of the named path |
| `symlink_target` | string | Yes | Symlink target path (only for symlinks) |

---

## Part File Naming

Parts are named `<base>.part<NNN>.<ext>`:

| Compression | Extension |
|-------------|-----------|
| `none` | `.tar` |
| `gzip` | `.tar.gz` |
| `zstd` | `.tar.zst` |
| `bzip2` | `.tar.bz2` |
| `lz4` | `.tar.lz4` |

Example with `base = "data"`, compression `zstd`, 3 parts:
```
data.part000.tar.zst
data.part001.tar.zst
data.part002.tar.zst
```

---

## Inspecting with jq

```bash
# Count files
jq '.header.total_files' index.arc.json

# List all paths
jq -r '.entries[].path' index.arc.json

# Find large files (>10 MB)
jq -r '.entries[] | select(.size > 10485760) | "\(.size)\t\(.path)"' index.arc.json | sort -rn

# List deduplicated files
jq -r '.entries[] | select(.dedup_of != null) | "\(.path) → dedup of \(.dedup_of)"' index.arc.json

# Show header
jq '.header' index.arc.json
```

---

## Version History

| Version | Changes |
|---------|---------|
| v3 | Adds `notes`, `dedup_of`, `zstd_level`, `part_bases`, Blake3 seal |
| v2 | Adds `created_at_human`, `total_symlinks`, `total_parts`, `compression`, `sha256`, `symlink_target` |
| v1 | Basic `path`, `size`, `mtime`, `tar_part` |
