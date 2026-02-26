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
//! `stats` — detailed archive statistics: per-extension, part sizes, ratios.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::index::ArchivumIndex;
use crate::output::OutputCtx;
use crate::scan::EntryType;
use crate::utils::human;

pub fn stats(index_path: &Path, out: &OutputCtx) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)?;
    let h = &idx.header;
    let index_dir = index_path.parent().unwrap_or(Path::new("."));
    let ext = h.compression.extension();

    // ── Extension breakdown ────────────────────────────────────────────────
    let mut ext_map: HashMap<String, (u64, u64)> = HashMap::new(); // ext → (count, bytes)
    for e in idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
    {
        let ext_str = e
            .path
            .extension()
            .map(|s| format!(".{}", s.to_string_lossy().to_lowercase()))
            .unwrap_or_else(|| "(no ext)".into());
        let entry = ext_map.entry(ext_str).or_default();
        entry.0 += 1;
        entry.1 += e.size;
    }
    let mut ext_vec: Vec<(String, u64, u64)> =
        ext_map.into_iter().map(|(k, (c, b))| (k, c, b)).collect();
    ext_vec.sort_by(|a, b| b.2.cmp(&a.2)); // sort by bytes desc

    // ── Part sizes (on disk) ───────────────────────────────────────────────
    let mut part_sizes: Vec<(u32, u64)> = vec![];
    let mut total_on_disk: u64 = 0;
    for part in 0..h.total_parts {
        let path = index_dir.join(format!("data.part{:03}{}", part, ext));
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        total_on_disk += size;
        part_sizes.push((part, size));
    }

    // ── Compression ratio ──────────────────────────────────────────────────
    let ratio = if total_on_disk > 0 {
        h.total_size as f64 / total_on_disk as f64
    } else {
        1.0
    };
    let saving_pct = if h.total_size > 0 {
        (1.0 - total_on_disk as f64 / h.total_size as f64) * 100.0
    } else {
        0.0
    };

    // ── Dedup savings ─────────────────────────────────────────────────────
    let dedup_count = idx.entries.iter().filter(|e| e.dedup_of.is_some()).count();
    let dedup_bytes: u64 = idx
        .entries
        .iter()
        .filter(|e| e.dedup_of.is_some())
        .map(|e| e.size)
        .sum();

    if out.json {
        let result = serde_json::json!({
            "header": {
                "created_at": h.created_at_human,
                "total_files": h.total_files,
                "total_dirs": h.total_dirs,
                "total_symlinks": h.total_symlinks,
                "total_size_bytes": h.total_size,
                "total_size_human": human(h.total_size),
                "total_parts": h.total_parts,
                "compression": h.compression.name(),
            },
            "on_disk_bytes": total_on_disk,
            "on_disk_human": human(total_on_disk),
            "compression_ratio": ratio,
            "saving_percent": saving_pct,
            "dedup_files": dedup_count,
            "dedup_bytes": dedup_bytes,
            "parts": part_sizes.iter().map(|(p, s)| serde_json::json!({"part": p, "size": s})).collect::<Vec<_>>(),
            "by_extension": ext_vec.iter().take(20).map(|(e, c, b)| {
                serde_json::json!({"ext": e, "count": c, "bytes": b})
            }).collect::<Vec<_>>()
        });
        out.raw(&serde_json::to_string_pretty(&result).unwrap());
        out.raw(
            "
",
        );
        return Ok(());
    }

    out.println(&"─".repeat(65).dimmed().to_string());
    out.println(&" ▲ Archive Statistics".cyan().bold().to_string());
    out.println(&"─".repeat(65).dimmed().to_string());
    out.println(&format!(
        "  Archive    : {}",
        index_path.display().to_string().yellow()
    ));
    out.println(&format!("  Created    : {}", h.created_at_human.dimmed()));
    out.println(&format!(
        "  Files      : {}  Dirs: {}  Symlinks: {}",
        h.total_files.to_string().cyan(),
        h.total_dirs.to_string().cyan(),
        h.total_symlinks.to_string().cyan()
    ));
    out.println(&format!("  Source size: {}", human(h.total_size).cyan()));
    out.println(&format!("  On-disk    : {}", human(total_on_disk).cyan()));
    out.println(&format!(
        "  Ratio      : {:.2}x  (saving: {:.1}%)",
        ratio, saving_pct
    ));
    if dedup_count > 0 {
        out.println(&format!(
            "  Deduped    : {} files  {} saved",
            dedup_count.to_string().yellow(),
            human(dedup_bytes).yellow()
        ));
    }

    // Parts table
    out.println("");
    out.println(&format!(
        "  {} ({} parts)",
        "Part sizes:".cyan().bold(),
        h.total_parts
    ));
    for (part, size) in &part_sizes {
        let path = index_dir.join(format!("data.part{:03}{}", part, ext));
        let exists = if path.exists() { "✓" } else { "✗" };
        out.println(&format!(
            "    {} part{:03}  {}",
            exists,
            part,
            human(*size).yellow()
        ));
    }

    // Extension table (top 15)
    out.println("");
    out.println(&format!("  {}", "Top file types by size:".cyan().bold()));
    out.println(&format!(
        "  {:<16} {:>8} {:>16}",
        "Extension".dimmed(),
        "Count".dimmed(),
        "Total Size".dimmed()
    ));
    out.println(&("  ".to_string() + &"─".repeat(40).dimmed().to_string()));
    for (ext_name, count, bytes) in ext_vec.iter().take(15) {
        out.println(&format!(
            "  {:<16} {:>8} {:>16}",
            ext_name,
            count,
            human(*bytes)
        ));
    }

    out.println(&"─".repeat(65).dimmed().to_string());
    Ok(())
}
