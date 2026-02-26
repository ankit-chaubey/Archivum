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
//! Diff an archive against a source directory — detects drift.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::checksum::hash_file;
use crate::index::ArchivumIndex;
use crate::output::OutputCtx;
use crate::scan::{scan_directory, EntryType};
use crate::utils::human;

pub fn diff(
    index_path: &Path,
    source: &Path,
    changed_only: bool,
    use_checksum: bool,
    out: &OutputCtx,
) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)?;

    out.println(&format!(
        "{} {} vs {}",
        "Diff:".cyan().bold(),
        index_path.display().to_string().yellow(),
        source.display().to_string().yellow()
    ));
    if use_checksum {
        out.println(&format!(
            "  {}",
            "Using SHA-256 checksum comparison".dimmed()
        ));
    }
    out.println("");

    let archived: HashMap<&Path, &crate::index::IndexEntry> = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.path.as_path(), e))
        .collect();

    let current = scan_directory(source, &[])?;
    let current_map: HashMap<&Path, &crate::scan::ScanEntry> = current
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.relative_path.as_path(), e))
        .collect();

    let mut added: Vec<(PathBuf, u64)> = vec![];
    let mut removed: Vec<PathBuf> = vec![];
    let mut modified: Vec<(PathBuf, String)> = vec![]; // (path, reason)
    let mut unchanged = 0usize;

    for (&path, se) in &current_map {
        if let Some(ae) = archived.get(path) {
            let size_changed = se.size != ae.size;
            let mtime_changed = se.mtime != ae.mtime;

            if size_changed || mtime_changed {
                let reason = if size_changed {
                    format!("size {} → {}", human(ae.size), human(se.size))
                } else {
                    "mtime changed".to_string()
                };
                modified.push((path.to_path_buf(), reason));
            } else if use_checksum {
                // Extra: compare by SHA-256 even if size/mtime match
                let full_path = source.join(path);
                match hash_file(&full_path) {
                    Ok(actual_hash) => {
                        let stored = ae.sha256.as_deref().unwrap_or("");
                        if !stored.is_empty() && actual_hash != stored {
                            modified.push((
                                path.to_path_buf(),
                                format!(
                                    "checksum mismatch ({}… vs {}…)",
                                    &stored[..8],
                                    &actual_hash[..8]
                                ),
                            ));
                        } else {
                            unchanged += 1;
                        }
                    }
                    Err(_) => unchanged += 1, // file unreadable — skip
                }
            } else {
                unchanged += 1;
            }
        } else {
            added.push((path.to_path_buf(), se.size));
        }
    }

    for &path in archived.keys() {
        if !current_map.contains_key(path) {
            removed.push(path.to_path_buf());
        }
    }

    if out.json {
        let result = serde_json::json!({
            "added":    added.iter().map(|(p, s)| serde_json::json!({"path": p, "size": s})).collect::<Vec<_>>(),
            "removed":  removed.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "modified": modified.iter().map(|(p, r)| serde_json::json!({"path": p, "reason": r})).collect::<Vec<_>>(),
            "unchanged": unchanged
        });
        out.raw(&serde_json::to_string_pretty(&result).unwrap());
        out.raw(
            "
",
        );
        return Ok(());
    }

    if !changed_only {
        out.println(&format!(
            "  {} {}",
            "Unchanged:".dimmed(),
            unchanged.to_string().dimmed()
        ));
    }

    for (path, size) in &added {
        out.println(&format!(
            "  {} {} ({})",
            "+ ADDED".green().bold(),
            path.display(),
            human(*size).green()
        ));
    }
    for path in &removed {
        out.println(&format!(
            "  {} {}",
            "- REMOVED".red().bold(),
            path.display()
        ));
    }
    for (path, reason) in &modified {
        out.println(&format!(
            "  {} {} — {}",
            "~ MODIFIED".yellow().bold(),
            path.display(),
            reason.dimmed()
        ));
    }

    out.println("");
    out.println(&"-".repeat(60).dimmed().to_string());
    out.println(&format!(
        "  Added: {}  Removed: {}  Modified: {}  Unchanged: {}",
        added.len().to_string().green(),
        removed.len().to_string().red(),
        modified.len().to_string().yellow(),
        unchanged.to_string().dimmed()
    ));
    out.println(&"-".repeat(60).dimmed().to_string());

    Ok(())
}
