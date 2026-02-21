use anyhow::{Context, Result};
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::copy;
use std::path::{Path, PathBuf};
use tar::Archive;

use crate::index::{ArchivumIndex, IndexEntry};
use crate::scan::EntryType;
use crate::utils::human;

pub fn restore(
    index_path: &Path,
    target: &Path,
    filter: Option<&str>,
    force: bool,
    _restore_permissions: bool,
) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)
        .with_context(|| format!("Cannot read index: {}", index_path.display()))?;
    let base_dir = index_path.parent().unwrap_or(Path::new("."));
    let algo = &idx.header.compression;
    let ext = algo.extension();

    let globset = build_filter(filter)?;

    println!(
        "{} {} -> {}",
        "Restoring:".cyan().bold(),
        index_path.display().to_string().yellow(),
        target.display().to_string().yellow()
    );
    println!();

    fs::create_dir_all(target)
        .with_context(|| format!("Cannot create target dir {}", target.display()))?;

    // First pass: create directories
    for entry in &idx.entries {
        if entry.entry_type != EntryType::Directory {
            continue;
        }
        if !matches_filter(&globset, &entry.path) {
            continue;
        }
        let dest = target.join(&entry.path);
        fs::create_dir_all(&dest)?;
        #[cfg(unix)]
        if _restore_permissions {
            apply_permissions(&dest, entry);
        }
    }

    // Second pass: symlinks
    for entry in &idx.entries {
        if entry.entry_type != EntryType::Symlink {
            continue;
        }
        if let Some(_target_link) = &entry.symlink_target {
            let link_path = target.join(&entry.path);
            if link_path.exists() {
                if force {
                    fs::remove_file(&link_path).ok();
                } else {
                    eprintln!("  skip (exists): {}", link_path.display());
                    continue;
                }
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(_target_link, &link_path)
                    .with_context(|| format!("Cannot create symlink {}", link_path.display()))?;
            }
            #[cfg(not(unix))]
            {
                let _ = &link_path;
                eprintln!("  symlinks skipped on non-Unix");
            }
        }
    }

    // Third pass: files grouped by tar_part for O(n+m) efficiency
    let mut by_part: HashMap<u32, Vec<&IndexEntry>> = HashMap::new();
    for entry in &idx.entries {
        if entry.entry_type != EntryType::File {
            continue;
        }
        if !matches_filter(&globset, &entry.path) {
            continue;
        }
        by_part.entry(entry.tar_part).or_default().push(entry);
    }

    let total_files: u64 = by_part.values().map(|v| v.len() as u64).sum();
    let total_bytes: u64 = by_part
        .values()
        .flat_map(|v| v.iter())
        .map(|e| e.size)
        .sum();

    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Restoring  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  ETA {eta}",
        )
        .unwrap()
        .progress_chars("ââââââââ "),
    );

    let mut sorted_parts: Vec<u32> = by_part.keys().cloned().collect();
    sorted_parts.sort_unstable();

    for part in sorted_parts {
        let entries = &by_part[&part];
        let part_path_buf = base_dir.join(format!("data.part{:03}{}", part, ext));

        let mut want: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        for e in entries {
            want.insert(e.path.clone(), e);
        }

        let reader = algo
            .wrap_reader(&part_path_buf)
            .with_context(|| format!("Cannot open part {}", part_path_buf.display()))?;
        let mut archive = Archive::new(reader);

        for item in archive.entries()? {
            let mut item = item?;
            let item_path = item.path()?.into_owned();

            if let Some(entry) = want.remove(&item_path) {
                let out_path = target.join(&entry.path);

                if out_path.exists() && !force {
                    eprintln!("  skip (exists): {}", out_path.display());
                    pb.inc(entry.size);
                    continue;
                }

                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p)?;
                }

                let mut out = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&out_path)
                    .with_context(|| format!("Cannot write {}", out_path.display()))?;

                copy(&mut item, &mut out)?;
                pb.inc(entry.size);

                #[cfg(unix)]
                if _restore_permissions {
                    apply_permissions(&out_path, entry);
                }
            }
        }
    }

    pb.finish_with_message(format!(
        "{}  ({} files, {})",
        "restore complete".green(),
        total_files,
        human(total_bytes)
    ));
    println!();
    println!(
        "  {} {}",
        "Restored to:".cyan().bold(),
        target.display().to_string().yellow()
    );

    Ok(())
}

pub fn extract_single(
    idx: &ArchivumIndex,
    base_dir: &Path,
    file: &Path,
    output: Option<&Path>,
) -> Result<()> {
    let entry = idx
        .entries
        .iter()
        .find(|e| e.path == file)
        .with_context(|| format!("File not found in archive: {}", file.display()))?;

    if entry.entry_type != EntryType::File {
        anyhow::bail!("Entry is not a regular file: {}", file.display());
    }

    let algo = &idx.header.compression;
    let ext = algo.extension();
    let part_path = base_dir.join(format!("data.part{:03}{}", entry.tar_part, ext));

    let reader = algo.wrap_reader(&part_path)?;
    let mut archive = Archive::new(reader);

    for item in archive.entries()? {
        let mut item = item?;
        if item.path()? == file {
            let out_path = match output {
                Some(p) => p.to_path_buf(),
                None => file
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| file.to_path_buf()),
            };

            if let Some(p) = out_path.parent() {
                if !p.as_os_str().is_empty() {
                    fs::create_dir_all(p)?;
                }
            }

            let mut out = File::create(&out_path)
                .with_context(|| format!("Cannot write {}", out_path.display()))?;
            copy(&mut item, &mut out)?;
            println!(
                "{} {}",
                "Extracted:".green().bold(),
                out_path.display().to_string().yellow()
            );
            return Ok(());
        }
    }

    anyhow::bail!("File not found in tar part: {}", file.display());
}

fn build_filter(pattern: Option<&str>) -> Result<Option<GlobSet>> {
    match pattern {
        None => Ok(None),
        Some(p) => {
            let mut b = GlobSetBuilder::new();
            b.add(Glob::new(p)?);
            Ok(Some(b.build()?))
        }
    }
}

fn matches_filter(gs: &Option<GlobSet>, path: &Path) -> bool {
    match gs {
        None => true,
        Some(g) => g.is_match(path),
    }
}

#[cfg(unix)]
fn apply_permissions(path: &Path, entry: &IndexEntry) {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = entry.unix_mode {
        let perms = fs::Permissions::from_mode(mode & 0o777);
        let _ = fs::set_permissions(path, perms);
    }
}
