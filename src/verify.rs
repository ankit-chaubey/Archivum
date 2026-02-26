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
//! Archive integrity verification — streaming SHA-256, no temp files.

use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::Path;

use crate::checksum::hash_reader;
use crate::index::ArchivumIndex;
use crate::output::OutputCtx;
use crate::scan::EntryType;

pub fn verify(index_path: &Path, continue_on_error: bool, out: &OutputCtx) -> Result<()> {
    let idx =
        ArchivumIndex::read(index_path).map_err(|e| anyhow::anyhow!("Cannot read index: {}", e))?;
    let index_dir = index_path.parent().unwrap_or(Path::new("."));

    out.println(&format!(
        "{} {}",
        "Verifying archive:".cyan().bold(),
        index_path.display().to_string().yellow()
    ));
    out.println("");

    // ── 1. Check tar parts exist ───────────────────────────────────────────
    let ext = idx.header.compression.extension();
    let mut all_parts_ok = true;
    for part in 0..idx.header.total_parts {
        let path = index_dir.join(format!("data.part{:03}{}", part, ext));
        if !path.exists() {
            let msg = format!("  {} {}", "MISSING".red().bold(), path.display());
            out.println(&msg);
            all_parts_ok = false;
            if !continue_on_error {
                anyhow::bail!("Missing tar part: {}", path.display());
            }
        } else {
            out.println(&format!(
                "  {}  {}",
                "OK".green(),
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    // ── 2. Checksum verification (streaming — no temp files) ───────────────
    let files_with_checksums: Vec<_> = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File && e.sha256.is_some() && e.dedup_of.is_none())
        .collect();

    if files_with_checksums.is_empty() {
        out.println("");
        out.println("  No checksums stored — archive was created without checksum support.");
        return Ok(());
    }

    let total_bytes: u64 = files_with_checksums.iter().map(|e| e.size).sum();
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Verifying  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  ETA {eta}",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    // Group by tar part for sequential reading
    let mut by_part: HashMap<u32, Vec<&crate::index::IndexEntry>> = HashMap::new();
    for e in &files_with_checksums {
        by_part.entry(e.tar_part).or_default().push(e);
    }

    let mut ok = 0usize;
    let mut bad = 0usize;
    let mut missing = 0usize;

    let mut sorted_parts: Vec<u32> = by_part.keys().cloned().collect();
    sorted_parts.sort_unstable();

    for part in sorted_parts {
        let entries = &by_part[&part];

        // Determine part path (using part_bases for incremental archives)
        let part_path = {
            let rep = entries[0];
            rep.part_path(index_dir, &idx.header)
        };

        if !part_path.exists() {
            missing += entries.len();
            pb.inc(entries.iter().map(|e| e.size).sum());
            continue;
        }

        // Build want map: path → expected sha256
        let mut want: HashMap<std::path::PathBuf, &str> = HashMap::new();
        for e in entries {
            want.insert(e.path.clone(), e.sha256.as_deref().unwrap());
        }

        let reader = idx.header.compression.wrap_reader(&part_path)?;
        let mut archive = tar::Archive::new(reader);

        for item in archive.entries()? {
            let mut item = item?;
            let item_path = item.path()?.into_owned();

            if let Some(&expected) = want.get(&item_path) {
                // ✅ FIX: stream hash directly — no temp file
                let actual = hash_reader(&mut item)?;

                let entry = entries.iter().find(|e| e.path == item_path).unwrap();

                if actual == expected {
                    ok += 1;
                } else {
                    bad += 1;
                    pb.suspend(|| {
                        eprintln!(
                            "  {} {} (expected {}… got {}…)",
                            "CORRUPT".red().bold(),
                            item_path.display(),
                            &expected[..12],
                            &actual[..12]
                        );
                    });
                    if !continue_on_error {
                        pb.finish_and_clear();
                        anyhow::bail!("Checksum mismatch for {}", item_path.display());
                    }
                }
                pb.inc(entry.size);
            }
        }
    }

    pb.finish_with_message("verification done");

    if out.json {
        let result = serde_json::json!({
            "status": if bad + missing == 0 { "PASS" } else { "FAIL" },
            "ok": ok,
            "corrupt": bad,
            "missing": missing,
            "all_parts_present": all_parts_ok
        });
        out.raw(&serde_json::to_string_pretty(&result).unwrap());
        out.raw(
            "
",
        );
    } else {
        out.println("");
        out.println(&"-".repeat(50).dimmed().to_string());
        let status_str = if bad + missing == 0 {
            "PASS".green().bold().to_string()
        } else {
            "FAIL".red().bold().to_string()
        };
        out.println(&format!(
            "  {}  OK: {}  CORRUPT: {}  MISSING: {}",
            status_str,
            ok.to_string().green(),
            if bad > 0 {
                bad.to_string().red().to_string()
            } else {
                bad.to_string().green().to_string()
            },
            if missing > 0 {
                missing.to_string().red().to_string()
            } else {
                missing.to_string().green().to_string()
            }
        ));
        out.println(&"-".repeat(50).dimmed().to_string());
    }

    if bad + missing > 0 && !continue_on_error {
        anyhow::bail!("{} file(s) failed verification", bad + missing);
    }

    Ok(())
}
