use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

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

    let archived: HashMap<&std::path::Path, &crate::index::IndexEntry> = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.path.as_path(), e))
        .collect();

    let current = scan_directory(source, &[])?;
    let current_map: HashMap<&std::path::Path, &crate::scan::ScanEntry> = current
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| (e.path.as_path(), e))
        .collect();

    let mut added = vec![];
    let mut removed = vec![];
    let mut modified = vec![];
    let mut unchanged = 0usize;

    for (path, se) in &current_map {
        if let Some(ae) = archived.get(path) {
            let size_changed = se.size != ae.size;
            let mtime_changed = se.mtime != ae.mtime;
            if size_changed || mtime_changed {
                modified.push((*path, ae.size, se.size));
            } else {
                unchanged += 1;
            }
        } else {
            added.push((*path, se.size));
        }
    }

    for (path, ae) in &archived {
        if !current_map.contains_key(path) {
            removed.push((*path, ae.size));
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
    for (path, _) in &removed {
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
