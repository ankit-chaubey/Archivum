// ─────────────────────────────────────────────────────────────────────────────
// Archivum v0.2.0
// Copyright 2026 Ankit Chaubey <ankitchaubey.dev@gmail.com>
// github.com/ankit-chaubey
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// All rights reserved 2026.
// ─────────────────────────────────────────────────────────────────────────────
//! `merge` — combine multiple archives into a single new archive.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::io::{self, copy};
use std::path::{Path, PathBuf};

use crate::compress::CompressionAlgo;
use crate::index::{ArchivumIndex, IndexEntry, IndexHeader, INDEX_VERSION};
use crate::output::OutputCtx;
use crate::scan::EntryType;
use crate::utils::{fmt_time, now};

// ─── A self-contained part writer that owns its builder+writer ────────────

struct PartWriter {
    builder: tar::Builder<Box<dyn io::Write>>,
    current_size: u64,
}

impl PartWriter {
    fn open(path: &Path, algo: &CompressionAlgo, zstd_level: i32) -> Result<Self> {
        let f =
            fs::File::create(path).with_context(|| format!("Cannot create {}", path.display()))?;
        let writer: Box<dyn io::Write> = algo.wrap_writer(f, zstd_level)?;
        Ok(Self {
            builder: tar::Builder::new(writer),
            current_size: 0,
        })
    }

    fn finish(mut self) -> Result<()> {
        self.builder
            .finish()
            .context("Failed to finalize tar part")?;
        Ok(())
    }
}

// ─── Main merge function ──────────────────────────────────────────────────

pub fn merge(
    index_paths: &[PathBuf],
    output_dir: &Path,
    split_bytes: u64,
    algo: &CompressionAlgo,
    zstd_level: i32,
    out: &OutputCtx,
) -> Result<()> {
    out.println(&format!(
        "{} {} archives → {}",
        "Merging:".cyan().bold(),
        index_paths.len().to_string().yellow(),
        output_dir.display().to_string().yellow()
    ));
    out.println("");

    if out.dry_run {
        for p in index_paths {
            out.dry(&format!("would merge: {}", p.display()));
        }
        out.dry(&format!("output: {}", output_dir.display()));
        return Ok(());
    }

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output dir {}", output_dir.display()))?;

    // ── Collect all entries, deduplicating by path ────────────────────────
    let mut work_list: Vec<(PathBuf, IndexEntry)> = vec![];
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    let mut total_skipped = 0usize;

    for idx_path in index_paths {
        let idx = ArchivumIndex::read(idx_path)
            .with_context(|| format!("Cannot read: {}", idx_path.display()))?;
        let dir = idx_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        out.println(&format!(
            "  Reading {} ({} files)",
            idx_path.display().to_string().yellow(),
            idx.header.total_files
        ));

        for entry in idx.entries {
            if entry.entry_type != EntryType::File || entry.dedup_of.is_some() {
                continue;
            }
            if seen_paths.contains(&entry.path) {
                total_skipped += 1;
                continue;
            }
            seen_paths.insert(entry.path.clone());
            work_list.push((dir.clone(), entry));
        }
    }

    if total_skipped > 0 {
        out.println(&format!(
            "  {} {} duplicate file(s) skipped",
            "Note:".yellow(),
            total_skipped
        ));
    }

    out.println(&format!(
        "
  Merging {} unique files...",
        work_list.len().to_string().cyan()
    ));

    // ── Write merged archive parts ────────────────────────────────────────
    let ext = algo.extension();
    let mut current_part: u32 = 0;
    let mut new_entries: Vec<IndexEntry> = vec![];

    let first_path = output_dir.join(format!("data.part{:03}{}", current_part, ext));
    let mut pw = PartWriter::open(&first_path, algo, zstd_level)?;

    for (src_dir, mut entry) in work_list {
        // Locate the source tar part
        let src_part_path = src_dir.join(format!(
            "data.part{:03}{}",
            entry.tar_part,
            algo.extension()
        ));

        let overhead = 512 + entry.size.div_ceil(512) * 512;

        // Rotate to new part if needed
        if pw.current_size > 0 && pw.current_size + overhead > split_bytes {
            pw.finish()?;
            current_part += 1;
            let next_path = output_dir.join(format!("data.part{:03}{}", current_part, ext));
            pw = PartWriter::open(&next_path, algo, zstd_level)?;
        }

        // Extract from old archive and write to new builder
        if let Ok(reader) = algo.wrap_reader(&src_part_path) {
            let mut src_archive = tar::Archive::new(reader);
            if let Ok(entries_iter) = src_archive.entries() {
                for item in entries_iter.flatten() {
                    let mut item = item;
                    let matches = item
                        .path()
                        .map(|p| p.as_ref() == entry.path.as_path())
                        .unwrap_or(false);
                    if matches {
                        let mut buf: Vec<u8> = Vec::with_capacity(entry.size as usize);
                        copy(&mut item, &mut buf)?;

                        let mut header = tar::Header::new_gnu();
                        header.set_path(&entry.path)?;
                        header.set_size(buf.len() as u64);
                        header.set_mode(entry.unix_mode.unwrap_or(0o644));
                        if let Some(mtime) = entry.mtime {
                            header.set_mtime(mtime);
                        }
                        header.set_cksum();

                        pw.builder.append(&header, &mut io::Cursor::new(&buf))?;
                        pw.current_size += overhead;
                        break;
                    }
                }
            }
        }

        entry.tar_part = current_part;
        entry.tar_base = None;
        new_entries.push(entry);
    }

    pw.finish()?;

    let total_parts = current_part + 1;
    let total_files = new_entries.len() as u64;
    let total_size: u64 = new_entries.iter().map(|e| e.size).sum();

    out.println(&format!(
        "  {} {} files in {} parts",
        "Merged:".green().bold(),
        total_files.to_string().cyan(),
        total_parts.to_string().cyan()
    ));

    let ts = now();
    let merged_idx = ArchivumIndex {
        header: IndexHeader {
            version: INDEX_VERSION,
            created_at_unix: ts,
            created_at_human: fmt_time(ts),
            total_files,
            total_dirs: 0,
            total_symlinks: 0,
            total_size,
            total_parts,
            compression: algo.clone(),
            zstd_level,
            notes: format!("Merged from {} archives", index_paths.len()),
            part_bases: vec![String::new()],
            _integrity: None,
        },
        entries: new_entries,
    };

    let index_path = output_dir.join("index.arc.json");
    merged_idx.write(&index_path)?;

    out.println(&format!(
        "
  {} {}",
        "Merged index written to:".green().bold(),
        index_path.display().to_string().yellow()
    ));

    Ok(())
}
