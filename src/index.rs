use anyhow::Result;
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::compress::CompressionAlgo;
use crate::scan::{EntryType, ScanEntry};
use crate::utils::{fmt_time, human, now};

pub const INDEX_VERSION: u32 = 2;

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
}

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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivumIndex {
    pub header: IndexHeader,
    pub entries: Vec<IndexEntry>,
}

impl ArchivumIndex {
    pub fn build(scan: Vec<ScanEntry>, compression: CompressionAlgo) -> Self {
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
            },
            entries,
        }
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let w = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(w, self)?;
        Ok(())
    }

    pub fn read(path: &Path) -> Result<Self> {
        let r = BufReader::new(File::open(path)?);
        let idx: Self = serde_json::from_reader(r)?;
        Ok(idx)
    }

    pub fn print_summary(&self, verbose: bool, filter: Option<&str>) -> Result<()> {
        let h = &self.header;
        println!("{}", "─".repeat(65).dimmed());
        println!(
            "{}  v{}",
            " ▲ Archivum Archive".black().on_cyan().bold(),
            h.version
        );
        println!("{}", "─".repeat(65).dimmed());
        println!("  Created   : {}", h.created_at_human.yellow());
        println!("  Files     : {}", h.total_files.to_string().cyan());
        println!("  Dirs      : {}", h.total_dirs.to_string().cyan());
        println!("  Symlinks  : {}", h.total_symlinks.to_string().cyan());
        println!("  Total size: {}", human(h.total_size).cyan());
        println!("  Parts     : {}", h.total_parts.to_string().cyan());
        println!("  Compress  : {}", h.compression.name().green());

        if verbose || filter.is_some() {
            let globset = filter
                .map(|f| -> Result<GlobSet> {
                    let mut b = GlobSetBuilder::new();
                    b.add(Glob::new(f)?);
                    Ok(b.build()?)
                })
                .transpose()?;

            println!("{}", "─".repeat(65).dimmed());
            println!(
                "  {:<8} {:<12} {:<10} {}",
                "PART".dimmed(),
                "SIZE".dimmed(),
                "TYPE".dimmed(),
                "PATH".dimmed()
            );
            println!("{}", "─".repeat(65).dimmed());

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
                println!(
                    "  {:<8} {:<12} {:<10} {}",
                    format!("part{:03}", e.tar_part),
                    human(e.size),
                    type_str,
                    e.path.display()
                );
            }
        }

        println!("{}", "─".repeat(65).dimmed());
        Ok(())
    }
}
