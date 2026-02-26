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
//! `update` — incremental archive: only re-archive new/changed files.
//!
//! Unchanged files remain in the old parts (referenced via part_bases).
//! Only changed/new files are written to new parts in the output directory.

use anyhow::{Context, Result};
use colored::Colorize;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::checksum::{compute_checksums, hash_file};
use crate::compress::CompressionAlgo;
use crate::index::{ArchivumIndex, IndexEntry, IndexHeader, INDEX_VERSION};
use crate::output::OutputCtx;
use crate::scan::{scan_directory, EntryType};
use crate::tar_writer::write_archive;
use crate::utils::{fmt_time, human, now};

pub fn update(
    old_index_path: &Path,
    source: &Path,
    output_dir: &Path,
    split_bytes: u64,
    split_files: usize,
    algo: &CompressionAlgo,
    zstd_level: i32,
    threads: usize,
    exclude: &[String],
    use_checksum: bool,
    out: &OutputCtx,
) -> Result<()> {
    out.println(&format!(
        "{} {} → {}",
        "Incremental update:".cyan().bold(),
        source.display().to_string().yellow(),
        output_dir.display().to_string().yellow()
    ));
    out.println("");

    let old_idx = ArchivumIndex::read(old_index_path)
        .with_context(|| format!("Cannot read old index: {}", old_index_path.display()))?;
    let old_index_dir = old_index_path.parent().unwrap_or(Path::new("."));

    // ── Build a map of old entries by path ────────────────────────────────
    let old_map: HashMap<&Path, &IndexEntry> = old_idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.path.as_path(), e))
        .collect();

    // ── Scan source ────────────────────────────────────────────────────────
    let scan = scan_directory(source, exclude)
        .with_context(|| format!("Failed to scan {}", source.display()))?;

    // ── Classify each file ────────────────────────────────────────────────
    let mut unchanged: Vec<IndexEntry> = vec![];
    let mut changed_paths: Vec<PathBuf> = vec![];
    let mut new_paths: Vec<PathBuf> = vec![];

    for se in &scan {
        if se.entry_type != EntryType::File {
            continue;
        }
        if let Some(old_entry) = old_map.get(se.relative_path.as_path()) {
            let size_match = se.size == old_entry.size;
            let mtime_match = se.mtime == old_entry.mtime;

            let is_unchanged = if use_checksum && old_entry.sha256.is_some() {
                // Full checksum comparison
                if size_match && mtime_match {
                    // Optimization: if size+mtime match, assume unchanged
                    true
                } else {
                    let actual = hash_file(&source.join(&se.relative_path)).unwrap_or_default();
                    actual == old_entry.sha256.as_deref().unwrap_or("")
                }
            } else {
                size_match && mtime_match
            };

            if is_unchanged {
                unchanged.push((*old_entry).clone());
            } else {
                changed_paths.push(se.relative_path.clone());
            }
        } else {
            new_paths.push(se.relative_path.clone());
        }
    }

    // Report
    out.println(&format!(
        "  Unchanged: {}  Changed: {}  New: {}  (source total: {} files)",
        unchanged.len().to_string().green(),
        changed_paths.len().to_string().yellow(),
        new_paths.len().to_string().cyan(),
        scan.iter().filter(|e| e.entry_type == EntryType::File).count()
    ));
    out.println("");

    if out.dry_run {
        for p in &changed_paths {
            out.dry(&format!("would re-archive: {}", p.display()));
        }
        for p in &new_paths {
            out.dry(&format!("would archive new: {}", p.display()));
        }
        return Ok(());
    }

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output dir {}", output_dir.display()))?;

    // ── Build a new scan for only changed+new files ────────────────────────
    let need_rearchive: std::collections::HashSet<&Path> = changed_paths
        .iter()
        .chain(new_paths.iter())
        .map(|p| p.as_path())
        .collect();

    let delta_scan: Vec<_> = scan
        .into_iter()
        .filter(|e| need_rearchive.contains(e.relative_path.as_path()))
        .collect();

    let delta_size: u64 = delta_scan
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| e.size)
        .sum();

    out.println(&format!(
        "  Re-archiving {} file(s) ({})",
        (changed_paths.len() + new_paths.len()).to_string().yellow(),
        human(delta_size)
    ));

    // ── Compute checksums for delta files ─────────────────────────────────
    let mut delta_idx = ArchivumIndex::build(delta_scan, algo.clone(), zstd_level);
    compute_checksums(source, &mut delta_idx, threads)?;

    // ── Write new delta parts ─────────────────────────────────────────────
    // Old parts stay in old_index_dir; new parts go to output_dir
    write_archive(source, output_dir, &mut delta_idx, split_bytes, split_files, algo, zstd_level)?;

    // ── Build merged index ────────────────────────────────────────────────
    // part_bases[0] = "" (output_dir itself, for new parts)
    // part_bases[1] = relative path from output_dir to old_index_dir (for old parts)
    let old_rel = relative_path(output_dir, old_index_dir);

    let mut all_entries: Vec<IndexEntry> = vec![];

    // Unchanged: re-point to old base (index 1)
    for mut e in unchanged {
        e.tar_base = Some(1);
        all_entries.push(e);
    }

    // Delta entries: stay in base 0 (output_dir)
    for e in delta_idx.entries {
        all_entries.push(e);
    }

    // Non-file entries from original index (dirs, symlinks)
    for e in &old_idx.entries {
        if e.entry_type != EntryType::File {
            all_entries.push(e.clone());
        }
    }

    // Dedup-of: rebuild from delta (new files only)
    let mut total_files = 0u64;
    let mut total_dirs = 0u64;
    let mut total_symlinks = 0u64;
    let mut total_size = 0u64;
    for e in &all_entries {
        match e.entry_type {
            EntryType::File => { total_files += 1; total_size += e.size; }
            EntryType::Directory => total_dirs += 1,
            EntryType::Symlink => total_symlinks += 1,
        }
    }

    let ts = now();
    let new_idx = ArchivumIndex {
        header: IndexHeader {
            version: INDEX_VERSION,
            created_at_unix: ts,
            created_at_human: fmt_time(ts),
            total_files,
            total_dirs,
            total_symlinks,
            total_size,
            total_parts: delta_idx.header.total_parts,
            compression: algo.clone(),
            zstd_level,
            notes: format!(
                "Incremental update from {}",
                old_index_path.display()
            ),
            part_bases: vec![
                String::new(),
                old_rel.to_string_lossy().into_owned(),
            ],
            _integrity: None,
        },
        entries: all_entries,
    };

    let new_index_path = output_dir.join("index.arc.json");
    new_idx.write(&new_index_path)?;

    out.println("");
    out.println(&format!(
        "  {} {}",
        "Incremental archive created:".green().bold(),
        new_index_path.display().to_string().yellow()
    ));
    out.println(&format!(
        "  Old parts referenced from: {}",
        old_index_dir.display().to_string().dimmed()
    ));

    Ok(())
}

/// Compute a relative path from `base` to `target`.
fn relative_path(base: &Path, target: &Path) -> PathBuf {
    // Attempt simple relative computation
    let base_abs = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let target_abs = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());

    let base_comps: Vec<_> = base_abs.components().collect();
    let target_comps: Vec<_> = target_abs.components().collect();

    let common = base_comps
        .iter()
        .zip(target_comps.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = base_comps.len() - common;
    let mut rel = PathBuf::new();
    for _ in 0..ups {
        rel.push("..");
    }
    for comp in &target_comps[common..] {
        rel.push(comp);
    }

    if rel.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        rel
    }
}
