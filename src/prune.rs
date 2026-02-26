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
//! `prune` — delete old archives, keeping a minimum number.

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::index::ArchivumIndex;
use crate::output::OutputCtx;
use crate::utils::now;

struct ArchiveInfo {
    dir: PathBuf,
    _index_path: PathBuf,
    created_at: u64,
}

pub fn prune(base_dir: &Path, keep_last: usize, max_age_days: u64, out: &OutputCtx) -> Result<()> {
    out.println(&format!(
        "{} {} (keep={}, max_age={}d)",
        "Pruning archives in:".cyan().bold(),
        base_dir.display().to_string().yellow(),
        keep_last,
        max_age_days
    ));
    out.println("");

    // ── Find all archives (subdirs containing index.arc.json) ──────────────
    let mut archives: Vec<ArchiveInfo> = vec![];

    if !base_dir.is_dir() {
        anyhow::bail!("Not a directory: {}", base_dir.display());
    }

    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let index_path = dir.join("index.arc.json");
        if !index_path.exists() {
            continue;
        }
        match ArchivumIndex::read(&index_path) {
            Ok(idx) => {
                archives.push(ArchiveInfo {
                    dir,
                    _index_path: index_path,
                    created_at: idx.header.created_at_unix,
                });
            }
            Err(e) => {
                out.println(&format!(
                    "  {} {} — {}",
                    "skip (unreadable):".dimmed(),
                    index_path.display(),
                    e
                ));
            }
        }
    }

    // Sort oldest first
    archives.sort_by_key(|a| a.created_at);

    out.println(&format!("  Found {} archive(s)", archives.len()));

    if archives.len() <= keep_last {
        out.println(&format!(
            "  {} Nothing to prune — count ({}) ≤ keep_last ({})",
            "OK".green().bold(),
            archives.len(),
            keep_last
        ));
        return Ok(());
    }

    let now_secs = now();
    let max_age_secs = max_age_days * 86400;

    let mut to_delete: Vec<&ArchiveInfo> = vec![];

    // Candidates to delete = all except the newest keep_last
    let candidates = &archives[..archives.len() - keep_last];

    for arch in candidates {
        let age_secs = now_secs.saturating_sub(arch.created_at);
        let too_old = max_age_days > 0 && age_secs >= max_age_secs;

        if too_old {
            to_delete.push(arch);
        } else if max_age_days == 0 {
            // No age limit — delete all beyond keep_last
            to_delete.push(arch);
        }
    }

    if to_delete.is_empty() {
        out.println(&"  Nothing qualified for deletion.".dimmed().to_string());
        return Ok(());
    }

    out.println(&format!(
        "  {} archive(s) to delete:",
        to_delete.len().to_string().red()
    ));

    for arch in &to_delete {
        let age_days = now_secs.saturating_sub(arch.created_at) / 86400;
        out.println(&format!(
            "    {} (age: {} days)",
            arch.dir.display().to_string().red(),
            age_days
        ));

        if out.dry_run {
            out.dry(&format!("would delete: {}", arch.dir.display()));
        } else {
            // Delete all archive parts and the index
            delete_archive(&arch.dir, out)?;
        }
    }

    if !out.dry_run {
        out.println(&format!(
            "
  {} Pruned {} archive(s)",
            "Done.".green().bold(),
            to_delete.len()
        ));
    }

    Ok(())
}

fn delete_archive(dir: &Path, out: &OutputCtx) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        let is_archive_file = name.starts_with("data.part")
            || name == "index.arc.json"
            || name == "index.arc.json.b3";

        if is_archive_file {
            fs::remove_file(&path).ok();
        }
    }

    // Remove dir if now empty
    if fs::read_dir(dir)?.next().is_none() {
        fs::remove_dir(dir).ok();
        out.println(&format!("  {} {}", "Deleted:".red().bold(), dir.display()));
    } else {
        out.println(&format!(
            "  {} {} (directory not empty — only archive files removed)",
            "Cleaned:".yellow(),
            dir.display()
        ));
    }

    Ok(())
}
