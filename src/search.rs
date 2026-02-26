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
//! `search` — search the index by glob or substring.

use anyhow::Result;
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

use crate::index::ArchivumIndex;
use crate::output::OutputCtx;
use crate::utils::human;

pub fn search(index_path: &Path, pattern: &str, out: &OutputCtx) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)?;

    // Only treat as glob if pattern contains glob metacharacters; otherwise use substring
    let is_glob = pattern.contains('*') || pattern.contains('?') || pattern.contains('[');
    let globset: Option<GlobSet> = if is_glob {
        Glob::new(pattern).ok().and_then(|g| {
            let mut b = GlobSetBuilder::new();
            b.add(g);
            b.build().ok()
        })
    } else {
        None
    };

    let matches: Vec<&crate::index::IndexEntry> = idx
        .entries
        .iter()
        .filter(|e| {
            let path_str = e.path.to_string_lossy();
            match &globset {
                Some(gs) => gs.is_match(&e.path),
                None => path_str.to_lowercase().contains(&pattern.to_lowercase()),
            }
        })
        .collect();

    if out.json {
        let json_matches: Vec<serde_json::Value> = matches
            .iter()
            .map(|e| {
                serde_json::json!({
                    "path": e.path,
                    "size": e.size,
                    "sha256": e.sha256,
                    "tar_part": e.tar_part,
                    "mtime": e.mtime
                })
            })
            .collect();
        out.raw(&serde_json::to_string_pretty(&json_matches).unwrap());
        out.raw(
            "
",
        );
        return Ok(());
    }

    out.println(&format!(
        "{} '{}' in {} — {} match(es)",
        "Search:".cyan().bold(),
        pattern.yellow(),
        index_path.display().to_string().dimmed(),
        matches.len().to_string().green()
    ));
    out.println(&"─".repeat(65).dimmed().to_string());
    out.println(&format!(
        "  {:<8} {:<12} {}",
        "PART".dimmed(),
        "SIZE".dimmed(),
        "PATH".dimmed()
    ));
    out.println(&"─".repeat(65).dimmed().to_string());

    for e in &matches {
        let dedup_tag = if e.dedup_of.is_some() {
            " [dedup]".dimmed().to_string()
        } else {
            String::new()
        };
        out.println(&format!(
            "  {:<8} {:<12} {}{}",
            format!("part{:03}", e.tar_part),
            human(e.size),
            e.path.display(),
            dedup_tag
        ));
    }

    out.println(&"─".repeat(65).dimmed().to_string());

    Ok(())
}
