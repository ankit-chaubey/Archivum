use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::Path;

use crate::checksum::hash_file;
use crate::index::ArchivumIndex;
use crate::scan::EntryType;

pub fn verify(index_path: &Path, continue_on_error: bool) -> Result<()> {
    let idx =
        ArchivumIndex::read(index_path).map_err(|e| anyhow::anyhow!("Cannot read index: {}", e))?;
    let base_dir = index_path.parent().unwrap_or(Path::new("."));
    let algo = &idx.header.compression;
    let ext = algo.extension();

    println!(
        "{} {}",
        "Verifying archive:".cyan().bold(),
        index_path.display().to_string().yellow()
    );
    println!();

    // Check tar parts exist
    for part in 0..idx.header.total_parts {
        let path = base_dir.join(format!("data.part{:03}{}", part, ext));
        if !path.exists() {
            let msg = format!("Missing tar part: {}", path.display());
            if continue_on_error {
                eprintln!("  MISSING {}", path.display());
            } else {
                anyhow::bail!(msg);
            }
        } else {
            println!(
                "  OK  {}",
                path.file_name().unwrap().to_string_lossy()
            );
        }
    }

    // Checksum verification
    let files_with_checksums: Vec<_> = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File && e.sha256.is_some())
        .collect();

    if files_with_checksums.is_empty() {
        println!();
        println!("  No checksums in index (archive created without checksum support)");
        return Ok(());
    }

    let total_bytes: u64 = files_with_checksums.iter().map(|e| e.size).sum();
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Verifying  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  ETA {eta}",
        )
        .unwrap()
        .progress_chars("ââââââââ "),
    );

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
        let part_path = base_dir.join(format!("data.part{:03}{}", part, ext));

        if !part_path.exists() {
            missing += entries.len();
            pb.inc(entries.iter().map(|e| e.size).sum());
            continue;
        }

        let mut want: HashMap<std::path::PathBuf, &str> = HashMap::new();
        for e in entries {
            want.insert(e.path.clone(), e.sha256.as_deref().unwrap());
        }

        let reader = algo.wrap_reader(&part_path)?;
        let mut archive = tar::Archive::new(reader);

        for item in archive.entries()? {
            let mut item = item?;
            let item_path = item.path()?.into_owned();

            if let Some(&expected) = want.get(&item_path) {
                let tmp = std::env::temp_dir().join(format!(
                    "archivum_verify_{}.tmp",
                    item_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
                {
                    let mut f = std::fs::File::create(&tmp)?;
                    std::io::copy(&mut item, &mut f)?;
                }

                let actual = hash_file(&tmp)?;
                let _ = std::fs::remove_file(&tmp);

                let entry = entries.iter().find(|e| e.path == item_path).unwrap();

                if actual == expected {
                    ok += 1;
                } else {
                    bad += 1;
                    pb.suspend(|| {
                        eprintln!(
                            "  CORRUPT {} (expected {} got {})",
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
    println!();
    println!("{}", "-".repeat(50).dimmed());
    println!(
        "  {} OK: {}  CORRUPT: {}  MISSING: {}",
        if bad + missing == 0 {
            "PASS".green().bold()
        } else {
            "FAIL".red().bold()
        },
        ok.to_string().green(),
        if bad > 0 {
            bad.to_string().red()
        } else {
            bad.to_string().green()
        },
        if missing > 0 {
            missing.to_string().red()
        } else {
            missing.to_string().green()
        }
    );
    println!("{}", "-".repeat(50).dimmed());

    if bad + missing > 0 && !continue_on_error {
        anyhow::bail!("{} file(s) failed verification", bad + missing);
    }

    Ok(())
}
