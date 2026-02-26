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
//! Restore archives to disk — with path-traversal protection and dry-run support.

use anyhow::{Context, Result};
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::copy;
use std::path::{Component, Path, PathBuf};
use tar::Archive;

use crate::index::{ArchivumIndex, IndexEntry};
use crate::output::OutputCtx;
use crate::scan::EntryType;
use crate::utils::human;

// ─── Path traversal guard ──────────────────────────────────────────────────

/// Ensure `path` does not escape `base` (no `..` components, absolute paths, etc.)
fn safe_join(base: &Path, path: &Path) -> Result<PathBuf> {
    // Reject absolute paths in the archive
    if path.is_absolute() {
        anyhow::bail!(
            "Path traversal blocked: archive entry is absolute: {}",
            path.display()
        );
    }

    // Reject any `..` components
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            anyhow::bail!(
                "Path traversal blocked: archive entry contains '..': {}",
                path.display()
            );
        }
    }

    let full = base.join(path);

    // Final canonicalization check (requires base to exist)
    if base.exists() {
        let canon_base = base
            .canonicalize()
            .with_context(|| format!("Cannot canonicalize base {}", base.display()))?;
        // We can't canonicalize full yet (it may not exist), so check the parent
        if let Some(parent) = full.parent() {
            if parent.exists() {
                let canon_parent = parent.canonicalize()?;
                if !canon_parent.starts_with(&canon_base) {
                    anyhow::bail!(
                        "Path traversal blocked: {} escapes target directory",
                        path.display()
                    );
                }
            }
        }
    }

    Ok(full)
}

// ─── Restore ───────────────────────────────────────────────────────────────

pub fn restore(
    index_path: &Path,
    target: &Path,
    filter: Option<&str>,
    force: bool,
    restore_permissions: bool,
    out: &OutputCtx,
) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)
        .with_context(|| format!("Cannot read index: {}", index_path.display()))?;
    let index_dir = index_path.parent().unwrap_or(Path::new("."));

    let globset = build_filter(filter)?;

    out.println(&format!(
        "{} {} -> {}",
        "Restoring:".cyan().bold(),
        index_path.display().to_string().yellow(),
        target.display().to_string().yellow()
    ));
    out.println("");

    if out.dry_run {
        out.dry(&format!("would create directory: {}", target.display()));
    } else {
        fs::create_dir_all(target)
            .with_context(|| format!("Cannot create target dir {}", target.display()))?;
    }

    // ── Pass 1: directories ────────────────────────────────────────────────
    for entry in &idx.entries {
        if entry.entry_type != EntryType::Directory {
            continue;
        }
        if !matches_filter(&globset, &entry.path) {
            continue;
        }
        let dest = safe_join(target, &entry.path)?;
        if out.dry_run {
            out.dry(&format!("mkdir {}", dest.display()));
        } else {
            fs::create_dir_all(&dest)?;
            #[cfg(unix)]
            if restore_permissions {
                apply_permissions(&dest, entry);
            }
        }
    }

    // ── Pass 2: symlinks ───────────────────────────────────────────────────
    for entry in &idx.entries {
        if entry.entry_type != EntryType::Symlink {
            continue;
        }
        if let Some(link_target) = &entry.symlink_target {
            let link_path = safe_join(target, &entry.path)?;
            if out.dry_run {
                out.dry(&format!(
                    "symlink {} -> {}",
                    link_path.display(),
                    link_target.display()
                ));
                continue;
            }
            if link_path.exists() {
                if force {
                    fs::remove_file(&link_path).ok();
                } else {
                    out.println(&format!(
                        "  {} {}",
                        "skip (exists):".dimmed(),
                        link_path.display()
                    ));
                    continue;
                }
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(link_target, &link_path)
                .with_context(|| format!("Cannot create symlink {}", link_path.display()))?;
            #[cfg(not(unix))]
            {
                let _ = &link_path;
                out.println("  symlinks skipped on non-Unix");
            }
        }
    }

    // ── Pass 3: deduped files (copy from first occurrence) ────────────────
    let mut dedup_done: HashMap<PathBuf, PathBuf> = HashMap::new(); // original_path → restored_path

    // ── Pass 4: regular files, grouped by tar_part ────────────────────────
    let mut by_part: HashMap<u32, Vec<&IndexEntry>> = HashMap::new();
    for entry in &idx.entries {
        if entry.entry_type != EntryType::File {
            continue;
        }
        if !matches_filter(&globset, &entry.path) {
            continue;
        }
        if entry.dedup_of.is_some() {
            continue; // handled after extraction
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
        .progress_chars("=> "),
    );

    let mut sorted_parts: Vec<u32> = by_part.keys().cloned().collect();
    sorted_parts.sort_unstable();

    for part in sorted_parts {
        let entries = &by_part[&part];

        let part_path = {
            let rep = entries[0];
            rep.part_path(index_dir, &idx.header)
        };

        let mut want: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        for e in entries {
            want.insert(e.path.clone(), e);
        }

        if out.dry_run {
            for e in entries {
                let out_path = safe_join(target, &e.path)?;
                out.dry(&format!(
                    "restore {} ({})",
                    out_path.display(),
                    human(e.size)
                ));
                pb.inc(e.size);
            }
            continue;
        }

        let reader = idx
            .header
            .compression
            .wrap_reader(&part_path)
            .with_context(|| format!("Cannot open part {}", part_path.display()))?;
        let mut archive = Archive::new(reader);

        for item in archive.entries()? {
            let mut item = item?;
            let item_path = item.path()?.into_owned();

            if let Some(entry) = want.remove(&item_path) {
                let out_path = safe_join(target, &entry.path)?;

                if out_path.exists() && !force {
                    out.println(&format!(
                        "  {} {}",
                        "skip (exists):".dimmed(),
                        out_path.display()
                    ));
                    pb.inc(entry.size);
                    continue;
                }

                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p)?;
                }

                let mut f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&out_path)
                    .with_context(|| format!("Cannot write {}", out_path.display()))?;

                copy(&mut item, &mut f)?;
                dedup_done.insert(entry.path.clone(), out_path.clone());
                pb.inc(entry.size);

                #[cfg(unix)]
                if restore_permissions {
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

    // ── Pass 5: restore deduped files by copying ───────────────────────────
    let dedup_entries: Vec<&IndexEntry> = idx
        .entries
        .iter()
        .filter(|e| {
            e.entry_type == EntryType::File
                && e.dedup_of.is_some()
                && matches_filter(&globset, &e.path)
        })
        .collect();

    for entry in dedup_entries {
        let original = entry.dedup_of.as_ref().unwrap();
        if let Some(src) = dedup_done.get(original) {
            let dest = safe_join(target, &entry.path)?;
            if out.dry_run {
                out.dry(&format!(
                    "copy dedup {} from {}",
                    dest.display(),
                    src.display()
                ));
            } else {
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p)?;
                }
                if dest.exists() && !force {
                    continue;
                }
                fs::copy(src, &dest)?;
            }
        }
    }

    out.println("");
    out.println(&format!(
        "  {} {}",
        "Restored to:".cyan().bold(),
        target.display().to_string().yellow()
    ));

    Ok(())
}

// ─── Extract single file ───────────────────────────────────────────────────

pub fn extract_single(
    idx: &ArchivumIndex,
    index_dir: &Path,
    file: &Path,
    output: Option<&Path>,
    out: &OutputCtx,
) -> Result<()> {
    let entry = idx
        .entries
        .iter()
        .find(|e| e.path == file)
        .with_context(|| format!("File not found in archive: {}", file.display()))?;

    if entry.entry_type != EntryType::File {
        anyhow::bail!("Entry is not a regular file: {}", file.display());
    }

    // Handle dedup: extract from original
    let (target_path, target_entry) = if let Some(ref orig) = entry.dedup_of {
        let orig_entry = idx
            .entries
            .iter()
            .find(|e| &e.path == orig)
            .with_context(|| format!("Dedup origin not found: {}", orig.display()))?;
        (orig.as_path(), orig_entry)
    } else {
        (file, entry)
    };

    let part_path = target_entry.part_path(index_dir, &idx.header);

    let reader = idx.header.compression.wrap_reader(&part_path)?;
    let mut archive = tar::Archive::new(reader);

    for item in archive.entries()? {
        let mut item = item?;
        if item.path()? == target_path {
            let out_path = match output {
                Some(p) => p.to_path_buf(),
                None => file
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| file.to_path_buf()),
            };

            if out.dry_run {
                out.dry(&format!(
                    "extract {} to {}",
                    file.display(),
                    out_path.display()
                ));
                return Ok(());
            }

            if let Some(p) = out_path.parent() {
                if !p.as_os_str().is_empty() {
                    fs::create_dir_all(p)?;
                }
            }

            let mut f = File::create(&out_path)
                .with_context(|| format!("Cannot write {}", out_path.display()))?;
            copy(&mut item, &mut f)?;
            out.println(&format!(
                "{} {}",
                "Extracted:".green().bold(),
                out_path.display().to_string().yellow()
            ));
            return Ok(());
        }
    }

    anyhow::bail!("File not found in tar part: {}", file.display());
}

// ─── Helpers ───────────────────────────────────────────────────────────────

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
