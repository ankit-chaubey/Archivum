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
//! `repair` — rebuild a missing or corrupt index.arc.json from tar parts.

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::compress::CompressionAlgo;
use crate::index::{ArchivumIndex, IndexEntry, IndexHeader, INDEX_VERSION};
use crate::output::OutputCtx;
use crate::scan::EntryType;
use crate::utils::{fmt_time, now};

pub fn repair(archive_dir: &Path, compression: &str, out: &OutputCtx) -> Result<()> {
    let algo = CompressionAlgo::parse(compression)
        .with_context(|| format!("Unknown compression: '{}'", compression))?;
    let ext = algo.extension();

    out.println(&format!(
        "{} {}",
        "Repairing index in:".cyan().bold(),
        archive_dir.display().to_string().yellow()
    ));
    out.println(&format!("  Compression: {}", algo.name().green()));
    out.println("");

    // ── Find all part files ────────────────────────────────────────────────
    let mut part_files: Vec<(u32, std::path::PathBuf)> = vec![];
    for entry in fs::read_dir(archive_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if name.starts_with("data.part") && name.ends_with(ext.trim_start_matches('.')) {
            // Parse part number
            let num_str = name
                .trim_start_matches("data.part")
                .trim_end_matches(ext.trim_start_matches('.'));
            // Handle the extra '.' in the extension
            let num_str = num_str.trim_end_matches('.');
            if let Ok(n) = num_str.parse::<u32>() {
                part_files.push((n, path));
            }
        }
    }

    // Handle extension correctly
    let mut found_parts: Vec<(u32, std::path::PathBuf)> = vec![];
    for entry in fs::read_dir(archive_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Match pattern: data.part###<ext>
        if let Some(rest) = name.strip_prefix("data.part") {
            if name.ends_with(ext) {
                let num_part = rest.trim_end_matches(ext.trim_start_matches('.'));
                let num_part = num_part.trim_end_matches('.');
                if let Ok(n) = num_part.parse::<u32>() {
                    found_parts.push((n, path));
                }
            }
        }
    }

    let _ = part_files; // suppress unused warning
    let mut found_parts = found_parts;
    found_parts.sort_by_key(|(n, _)| *n);

    if found_parts.is_empty() {
        anyhow::bail!(
            "No {} parts found in {}. Check --compression flag.",
            ext,
            archive_dir.display()
        );
    }

    out.println(&format!("  Found {} part(s)", found_parts.len()));

    // ── Scan each part and collect entries ────────────────────────────────
    let mut entries: Vec<IndexEntry> = vec![];
    let mut total_files = 0u64;
    let mut total_size = 0u64;

    for (part_num, part_path) in &found_parts {
        out.println(&format!(
            "  Scanning {} ...",
            part_path.file_name().unwrap().to_string_lossy().yellow()
        ));

        let reader = match algo.wrap_reader(part_path) {
            Ok(r) => r,
            Err(e) => {
                out.eprintln(&format!("  Cannot read {}: {}", part_path.display(), e));
                continue;
            }
        };

        let mut archive = tar::Archive::new(reader);

        for item in archive.entries()? {
            let item = match item {
                Ok(i) => i,
                Err(e) => {
                    out.eprintln(&format!("  Entry error in part {}: {}", part_num, e));
                    continue;
                }
            };

            let header = item.header();
            let path = item.path()?.into_owned();
            let size = header.size()?;
            let mtime = header.mtime().ok();
            let mode = header.mode().ok();

            let entry_type = match header.entry_type() {
                tar::EntryType::Regular | tar::EntryType::Continuous => EntryType::File,
                tar::EntryType::Directory => EntryType::Directory,
                tar::EntryType::Symlink => EntryType::Symlink,
                _ => continue,
            };

            let symlink_target = if entry_type == EntryType::Symlink {
                header.link_name().ok().flatten().map(|l| l.into_owned())
            } else {
                None
            };

            if entry_type == EntryType::File {
                total_files += 1;
                total_size += size;
            }

            entries.push(IndexEntry {
                path,
                entry_type,
                size,
                mtime,
                unix_mode: mode,
                sha256: None, // can't recover checksums without source
                tar_part: *part_num,
                symlink_target,
                tar_base: None,
                dedup_of: None,
            });
        }
    }

    out.println(&format!(
        "  Recovered {} file entries from {} parts",
        total_files.to_string().green(),
        found_parts.len()
    ));
    out.println("  Note: SHA-256 checksums cannot be recovered without the source.");
    out.println("");

    // ── Build new index ────────────────────────────────────────────────────
    let ts = now();
    let idx = ArchivumIndex {
        header: IndexHeader {
            version: INDEX_VERSION,
            created_at_unix: ts,
            created_at_human: fmt_time(ts),
            total_files,
            total_dirs: entries
                .iter()
                .filter(|e| e.entry_type == EntryType::Directory)
                .count() as u64,
            total_symlinks: entries
                .iter()
                .filter(|e| e.entry_type == EntryType::Symlink)
                .count() as u64,
            total_size,
            total_parts: found_parts.last().map(|(n, _)| n + 1).unwrap_or(0),
            compression: algo,
            zstd_level: 3,
            notes: "Repaired index — checksums not available".into(),
            part_bases: vec![String::new()],
            _integrity: None,
        },
        entries,
    };

    if out.dry_run {
        out.dry("would write index.arc.json");
        return Ok(());
    }

    let index_path = archive_dir.join("index.arc.json");
    idx.write(&index_path)?;

    out.println(&format!(
        "{} {}",
        "Repaired index written to:".green().bold(),
        index_path.display().to_string().yellow()
    ));

    Ok(())
}
