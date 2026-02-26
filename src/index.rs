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
//! Index format v3 — adds notes, dedup, multi-base part refs, blake3 integrity.

use anyhow::Result;
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::compress::CompressionAlgo;
use crate::output::OutputCtx;
use crate::scan::{EntryType, ScanEntry};
use crate::utils::{fmt_time, human, now};

pub const INDEX_VERSION: u32 = 3;

// ─── Header ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexHeader {
    pub version: u32,
    pub created_at_unix: u64,
    pub created_at_human: String,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_symlinks: u64,
    pub total_size: u64,
    pub total_parts: u32,
    pub compression: CompressionAlgo,
    /// Zstd compression level (stored for correct decompression hints)
    #[serde(default = "default_zstd_level")]
    pub zstd_level: i32,
    /// Optional user-provided description
    #[serde(default)]
    pub notes: String,
    /// Base directories for parts (relative to index file location).
    /// Index 0 = same directory as index. Used by incremental update.
    #[serde(default = "default_part_bases")]
    pub part_bases: Vec<String>,
    /// blake3 hash of the index JSON (written to companion .b3 file)
    #[serde(skip)]
    pub _integrity: Option<String>,
}

fn default_zstd_level() -> i32 {
    3
}
fn default_part_bases() -> Vec<String> {
    vec![String::new()]
}

// ─── Entry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: PathBuf,
    pub entry_type: EntryType,
    pub size: u64,
    pub mtime: Option<u64>,
    pub unix_mode: Option<u32>,
    pub sha256: Option<String>,
    pub tar_part: u32,
    pub symlink_target: Option<PathBuf>,
    /// Which entry in header.part_bases this part lives under (None = 0)
    #[serde(default)]
    pub tar_base: Option<u32>,
    /// If Some, this file is a dedup of the referenced path (not stored in tar)
    #[serde(default)]
    pub dedup_of: Option<PathBuf>,
}

impl IndexEntry {
    /// Resolve the absolute path of this entry's tar part.
    pub fn part_path(&self, index_dir: &Path, header: &IndexHeader) -> PathBuf {
        let base_idx = self.tar_base.unwrap_or(0) as usize;
        let base = header
            .part_bases
            .get(base_idx)
            .map(|s| s.as_str())
            .unwrap_or("");
        let dir = if base.is_empty() {
            index_dir.to_path_buf()
        } else {
            index_dir.join(base)
        };
        dir.join(format!(
            "data.part{:03}{}",
            self.tar_part,
            header.compression.extension()
        ))
    }
}

// ─── Archive index ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivumIndex {
    pub header: IndexHeader,
    pub entries: Vec<IndexEntry>,
}

impl ArchivumIndex {
    pub fn build(scan: Vec<ScanEntry>, compression: CompressionAlgo, zstd_level: i32) -> Self {
        let mut files = 0u64;
        let mut dirs = 0u64;
        let mut symlinks = 0u64;
        let mut size = 0u64;

        let entries = scan
            .into_iter()
            .map(|e| {
                match e.entry_type {
                    EntryType::File => {
                        files += 1;
                        size += e.size;
                    }
                    EntryType::Directory => dirs += 1,
                    EntryType::Symlink => symlinks += 1,
                }
                IndexEntry {
                    path: e.relative_path,
                    entry_type: e.entry_type,
                    size: e.size,
                    mtime: e.mtime,
                    unix_mode: e.unix_mode,
                    sha256: None,
                    tar_part: 0,
                    symlink_target: e.symlink_target,
                    tar_base: None,
                    dedup_of: None,
                }
            })
            .collect();

        let ts = now();
        Self {
            header: IndexHeader {
                version: INDEX_VERSION,
                created_at_unix: ts,
                created_at_human: fmt_time(ts),
                total_files: files,
                total_dirs: dirs,
                total_symlinks: symlinks,
                total_size: size,
                total_parts: 0,
                compression,
                zstd_level,
                notes: String::new(),
                part_bases: vec![String::new()],
                _integrity: None,
            },
            entries,
        }
    }

    /// Serialize to JSON and write, plus a companion .b3 integrity file.
    pub fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self)?;

        // Write index JSON
        std::fs::write(path, &json)?;

        // Write blake3 integrity companion file
        let hash = blake3::hash(&json);
        let b3_path = path.with_extension("json.b3");
        std::fs::write(&b3_path, hash.to_hex().as_str())?;

        Ok(())
    }

    /// Read and optionally verify blake3 integrity.
    pub fn read(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;

        // Verify integrity if companion file exists
        let b3_path = path.with_extension("json.b3");
        if b3_path.exists() {
            let stored_hex = std::fs::read_to_string(&b3_path)?;
            let stored_hex = stored_hex.trim();
            let actual = blake3::hash(&bytes);
            if actual.to_hex().as_str() != stored_hex {
                anyhow::bail!(
                    "Index integrity check FAILED for {}.
  \
                     The index may have been tampered with or corrupted.
  \
                     Expected: {}
  Got:      {}",
                    path.display(),
                    stored_hex,
                    actual.to_hex()
                );
            }
        }

        let r = BufReader::new(std::io::Cursor::new(bytes));
        let idx: Self = serde_json::from_reader(r)?;
        Ok(idx)
    }

    // ─── Pretty print ─────────────────────────────────────────────────────

    pub fn print_summary(
        &self,
        verbose: bool,
        filter: Option<&str>,
        out: &OutputCtx,
    ) -> Result<()> {
        let h = &self.header;
        out.println(&"─".repeat(65).dimmed().to_string());
        out.println(&format!(
            "{}  v{}",
            " ▲ Archivum Archive".black().on_cyan().bold(),
            h.version
        ));
        out.println(&"─".repeat(65).dimmed().to_string());
        out.println(&format!("  Created   : {}", h.created_at_human.yellow()));
        if !h.notes.is_empty() {
            out.println(&format!("  Notes     : {}", h.notes.cyan()));
        }
        out.println(&format!(
            "  Files     : {}",
            h.total_files.to_string().cyan()
        ));
        out.println(&format!(
            "  Dirs      : {}",
            h.total_dirs.to_string().cyan()
        ));
        out.println(&format!(
            "  Symlinks  : {}",
            h.total_symlinks.to_string().cyan()
        ));
        out.println(&format!("  Total size: {}", human(h.total_size).cyan()));
        out.println(&format!(
            "  Parts     : {}",
            h.total_parts.to_string().cyan()
        ));
        out.println(&format!("  Compress  : {}", h.compression.name().green()));
        if h.compression == CompressionAlgo::Zstd {
            out.println(&format!(
                "  Zstd lvl  : {}",
                h.zstd_level.to_string().green()
            ));
        }

        // Check for deduped files
        let deduped = self.entries.iter().filter(|e| e.dedup_of.is_some()).count();
        if deduped > 0 {
            out.println(&format!(
                "  Deduped   : {} files",
                deduped.to_string().yellow()
            ));
        }

        if verbose || filter.is_some() {
            let globset = filter
                .map(|f| -> Result<GlobSet> {
                    let mut b = GlobSetBuilder::new();
                    b.add(Glob::new(f)?);
                    Ok(b.build()?)
                })
                .transpose()?;

            out.println(&"─".repeat(65).dimmed().to_string());
            out.println(&format!(
                "  {:<8} {:<12} {:<10} {}",
                "PART".dimmed(),
                "SIZE".dimmed(),
                "TYPE".dimmed(),
                "PATH".dimmed()
            ));
            out.println(&"─".repeat(65).dimmed().to_string());

            for e in &self.entries {
                if let Some(gs) = &globset {
                    if !gs.is_match(&e.path) {
                        continue;
                    }
                }
                let type_str = match e.entry_type {
                    EntryType::File => "file".green(),
                    EntryType::Directory => "dir".blue(),
                    EntryType::Symlink => "symlink".yellow(),
                };
                let dedup_tag = if e.dedup_of.is_some() {
                    " [dedup]".dimmed().to_string()
                } else {
                    String::new()
                };
                out.println(&format!(
                    "  {:<8} {:<12} {:<10} {}{}",
                    format!("part{:03}", e.tar_part),
                    human(e.size),
                    type_str,
                    e.path.display(),
                    dedup_tag
                ));
            }
        }

        out.println(&"─".repeat(65).dimmed().to_string());
        Ok(())
    }

    pub fn print_summary_json(&self) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(self)?);
        Ok(())
    }
}
