use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::index::ArchivumIndex;
use crate::scan::{scan_directory, EntryType};
use crate::utils::human;

pub fn diff(index_path: &Path, source: &Path, changed_only: bool) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)?;

    println!(
        "{} {} vs {}",
        "Diff:".cyan().bold(),
        index_path.display().to_string().yellow(),
        source.display().to_string().yellow()
    );
    println!();

    // Build map: relative_path → IndexEntry  (from archive)
    let archived: HashMap<&Path, &crate::index::IndexEntry> = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.path.as_path(), e))
        .collect();

    // Scan current source directory
    let current = scan_directory(source, &[])?;
    let current_map: HashMap<&Path, &crate::scan::ScanEntry> = current
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.relative_path.as_path(), e))
        .collect();

    // Use PathBuf (owned) in result vecs to avoid &&Path type-inference issues
    let mut added: Vec<(PathBuf, u64)> = vec![];
    let mut removed: Vec<PathBuf> = vec![];
    let mut modified: Vec<(PathBuf, u64, u64)> = vec![];
    let mut unchanged = 0usize;

    for (&path, se) in &current_map {
        if let Some(ae) = archived.get(path) {
            if se.size != ae.size || se.mtime != ae.mtime {
                modified.push((path.to_path_buf(), ae.size, se.size));
            } else {
                unchanged += 1;
            }
        } else {
            added.push((path.to_path_buf(), se.size));
        }
    }

    for (&path, _ae) in &archived {
        if !current_map.contains_key(path) {
            removed.push(path.to_path_buf());
        }
    }

    if !changed_only {
        println!(
            "  {} {}",
            "Unchanged:".dimmed(),
            unchanged.to_string().dimmed()
        );
    }

    for (path, size) in &added {
        println!(
            "  {} {} ({})",
            "+ ADDED".green().bold(),
            path.display(),
            human(*size).green()
        );
    }
    for path in &removed {
        println!("  {} {}", "- REMOVED".red().bold(), path.display());
    }
    for (path, old, new) in &modified {
        println!(
            "  {} {} ({} → {})",
            "~ MODIFIED".yellow().bold(),
            path.display(),
            human(*old),
            human(*new)
        );
    }

    println!();
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  Added: {}  Removed: {}  Modified: {}  Unchanged: {}",
        added.len().to_string().green(),
        removed.len().to_string().red(),
        modified.len().to_string().yellow(),
        unchanged.to_string().dimmed()
    );
    println!("{}", "─".repeat(60).dimmed());

    Ok(())
}
