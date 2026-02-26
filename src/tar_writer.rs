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
//! Writes source files into split, optionally compressed tar parts.

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tar::Builder;

use crate::compress::CompressionAlgo;
use crate::index::ArchivumIndex;
use crate::scan::EntryType;
use crate::utils::human;

pub fn write_archive(
    root: &Path,
    out_dir: &Path,
    idx: &mut ArchivumIndex,
    split_bytes: u64,
    split_files: usize,  // 0 = disabled
    algo: &CompressionAlgo,
    zstd_level: i32,
) -> Result<()> {
    let total_bytes: u64 = idx.header.total_size;
    let ext = algo.extension();

    // Pass 1: assign each file to a tar part (respecting split_bytes and split_files)
    let mut current_part: u32 = 0;
    let mut current_size: u64 = 0;
    let mut current_file_count: usize = 0;

    let file_indices: Vec<usize> = idx
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.entry_type == EntryType::File && e.dedup_of.is_none())
        .map(|(i, _)| i)
        .collect();

    for &ei in &file_indices {
        let size = idx.entries[ei].size;
        let overhead = 512 + size.div_ceil(512) * 512;

        // Check split conditions
        let byte_overflow = current_size > 0 && current_size + overhead > split_bytes;
        let file_overflow = split_files > 0 && current_file_count >= split_files;

        if byte_overflow || file_overflow {
            current_part += 1;
            current_size = 0;
            current_file_count = 0;
        }

        idx.entries[ei].tar_part = current_part;
        current_size += overhead;
        current_file_count += 1;
    }

    let total_parts = if file_indices.is_empty() { 0 } else { current_part + 1 };
    idx.header.total_parts = total_parts;

    if total_parts == 0 {
        return Ok(());
    }

    // Pass 2: write each part
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Archiving  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  ETA {eta}",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    for part in 0..total_parts {
        let part_path = out_dir.join(format!("data.part{:03}{}", part, ext));
        write_part(root, idx, part, &part_path, algo, zstd_level, &pb)?;
    }

    pb.finish_with_message(format!(
        "{}  ({} parts, {})",
        "archive written".green(),
        total_parts,
        human(total_bytes)
    ));

    Ok(())
}

fn write_part(
    root: &Path,
    idx: &ArchivumIndex,
    part: u32,
    part_path: &Path,
    algo: &CompressionAlgo,
    zstd_level: i32,
    pb: &ProgressBar,
) -> Result<()> {
    let file = File::create(part_path)
        .with_context(|| format!("Cannot create {}", part_path.display()))?;

    let mut writer: Box<dyn Write> = algo.wrap_writer(file, zstd_level)?;
    let mut builder = Builder::new(&mut writer);

    for entry in idx.entries.iter().filter(|e| {
        e.entry_type == EntryType::File
            && e.tar_part == part
            && e.dedup_of.is_none()
    }) {
        let full = root.join(&entry.path);
        let mut f =
            File::open(&full).with_context(|| format!("Cannot open {}", full.display()))?;
        builder
            .append_file(&entry.path, &mut f)
            .with_context(|| format!("Failed to append {}", entry.path.display()))?;
        pb.inc(entry.size);
    }

    builder.finish().context("Failed to finalize tar part")?;
    drop(builder);
    drop(writer);

    Ok(())
}
