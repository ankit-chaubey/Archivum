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
//! Parallel SHA-256 checksumming using Rayon.

use anyhow::Result;
use colored::Colorize;
use hex::encode;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::index::ArchivumIndex;
use crate::scan::EntryType;

pub fn compute_checksums(root: &Path, idx: &mut ArchivumIndex, num_threads: usize) -> Result<()> {
    let total: u64 = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File && e.dedup_of.is_none())
        .map(|e| e.size)
        .sum();

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Checksums  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  {elapsed}",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    // Build work list: (index_position, abs_path, size)
    let work: Vec<(usize, std::path::PathBuf, u64)> = idx
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.entry_type == EntryType::File && e.dedup_of.is_none())
        .map(|(i, e)| (i, root.join(&e.path), e.size))
        .collect();

    // Configure rayon thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads.max(1))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build thread pool: {}", e))?;

    let pb_arc = Arc::new(pb);
    let results: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(Vec::new()));

    pool.install(|| {
        work.par_iter()
            .map(|(idx_pos, path, _size)| -> Result<(usize, String)> {
                let hash = hash_file(path)?;
                Ok((*idx_pos, hash))
            })
            .for_each(|result| match result {
                Ok((pos, hash)) => {
                    let size = idx.entries[pos].size;
                    results.lock().unwrap().push((pos, hash));
                    pb_arc.inc(size);
                }
                Err(e) => {
                    pb_arc.suspend(|| eprintln!("  checksum error: {}", e));
                }
            });
    });

    pb_arc.finish_with_message("checksums done".green().to_string());

    // Write results back
    let res = results.lock().unwrap();
    for (i, hash) in res.iter() {
        idx.entries[*i].sha256 = Some(hash.clone());
    }

    // ── Deduplication: mark duplicate files by SHA-256 ────────────────────
    let mut seen: std::collections::HashMap<String, std::path::PathBuf> =
        std::collections::HashMap::new();

    for entry in idx.entries.iter_mut() {
        if entry.entry_type != EntryType::File {
            continue;
        }
        if let Some(ref hash) = entry.sha256.clone() {
            if let Some(first_path) = seen.get(hash) {
                entry.dedup_of = Some(first_path.clone());
            } else {
                seen.insert(hash.clone(), entry.path.clone());
            }
        }
    }

    Ok(())
}

/// Stream-hash a file using SHA-256. No temp files.
pub fn hash_file(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 131072]; // 128 KiB chunks
    let file =
        File::open(path).map_err(|e| anyhow::anyhow!("Cannot open {}: {}", path.display(), e))?;
    let mut reader = BufReader::new(file);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(encode(hasher.finalize()))
}

/// Stream-hash from an arbitrary reader (used in verify to avoid temp files).
pub fn hash_reader<R: Read>(reader: &mut R) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 131072];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(encode(hasher.finalize()))
}
