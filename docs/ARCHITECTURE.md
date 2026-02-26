# Archivum — Architecture

This document describes the internal structure of the Archivum codebase.

---

## Module Overview

```
src/
├── main.rs          CLI entry point — clap parsing, command dispatch
├── output.rs        OutputCtx — unified quiet/json/dry-run/log routing
├── config.rs        config.toml loading, setup wizard
│
├── scan.rs          Directory traversal (WalkDir), symlink detection, excludes
├── checksum.rs      Parallel SHA-256 (Rayon) + Blake3 sealing
├── compress.rs      Compression abstraction: none/gzip/zstd/bzip2/lz4
├── tar_writer.rs    Two-pass tar writing: size assignment → write
│
├── index.rs         ArchivumIndex v3: build/read/write/print/seal
├── restore.rs       Grouped restore engine + single-file extract
├── verify.rs        Part existence + SHA-256 + Blake3 index verification
├── diff.rs          Archive vs live-source drift detection
│
├── search.rs        Glob + substring search over index entries
├── stats.rs         Compression ratio, extension stats, dedup savings
├── cat.rs           Stream single file to stdout
│
├── update.rs        Incremental archive (new/modified only)
├── merge.rs         Multi-archive merge into one
├── prune.rs         Age + count based archive pruning
├── repair.rs        Index reconstruction from orphaned tar parts
│
├── completions.rs   Shell completion generation (bash/zsh/fish)
└── utils.rs         Formatting helpers, timestamps, print_banner
```

---

## Data Flow

### Create

```
Source directory
      │
      ▼
scan::scan_directory()          ← WalkDir + glob excludes
      │
      ▼
index::ArchivumIndex::build()   ← assign tar parts (two-pass)
      │
      ▼
checksum::compute_checksums()   ← parallel SHA-256 via Rayon
      │
      ▼
tar_writer::write_archive()     ← stream source → tar → compress
      │
      ▼
index.write()                   ← index.arc.json
checksum::seal_index()          ← index.arc.json.b3 (Blake3)
```

### Restore

```
index.arc.json → read + verify Blake3 seal
      │
      ▼
Group entries by tar_part        ← O(n) grouping
      │
      ▼
For each part (opened once):
  ├── Decompress stream
  ├── Iterate tar entries
  └── Write matching files → target/
```

### Verify

```
index.arc.json.b3 → check Blake3 seal
      │
      ▼
For each expected part:
  └── Check file exists on disk
      │
      ▼
For each file entry:
  └── Recompute SHA-256 from tar → compare to stored value
```

---

## Key Design Decisions

### Two-Pass Tar Writing

Files cannot be assigned to parts and written in one pass because the part assignment depends on cumulative sizes. Instead:

1. **Pass 1**: Walk all entries, compute cumulative sizes, assign `tar_part` index
2. **Pass 2**: Open each part file and stream the actual file bytes

This eliminates borrow checker issues and allows clean part boundary logic.

### OutputCtx

Every command receives an `&OutputCtx` reference. All output is routed through it:

```rust
pub struct OutputCtx {
    pub json: bool,
    pub quiet: bool,
    pub dry_run: bool,
    log: Option<Arc<Mutex<File>>>,
}
```

- `out.println(s)` — prints unless quiet; always logs to file
- `out.eprintln(s)` — always prints to stderr; always logs
- `out.raw(s)` — always prints (for JSON / cat output)
- `out.dry(s)` — prints `[dry-run] …` unless quiet

This ensures `--quiet`, `--log-file`, and `--json` work consistently across every command.

### Blake3 Index Seal

After writing `index.arc.json`, the Blake3 hash of its content is written to `index.arc.json.b3`. Before any verify or restore operation, this seal is checked. Any modification of the index — even a single byte — is detected immediately.

### Deduplication

When `--dedup` is enabled:

1. As checksums are computed, a `HashMap<sha256, path>` is maintained
2. If a file's SHA-256 matches a previously seen file, its `dedup_of` field is set to that path
3. Deduped files are included in the index but **not** written to the tar
4. During restore, dedup files are restored by copying from the already-restored original

### Path Traversal Guard

During restore, every entry path is checked for `..` components:

```rust
if entry.path.components().any(|c| c == Component::ParentDir) {
    bail!("Path traversal attempt: {}", entry.path.display());
}
```

This prevents a malicious archive from writing outside the target directory.

---

## Adding a New Command

1. Create `src/my_command.rs`
2. Add the Apache 2.0 header
3. Implement `pub fn my_command(..., out: &OutputCtx) -> Result<()>`
4. Add `mod my_command;` to `main.rs`
5. Add the `Commands::MyCommand { ... }` variant to the enum
6. Wire up the dispatch arm in the `match cli.command` block
7. Update `README.md` and `docs/COMMANDS.md`
